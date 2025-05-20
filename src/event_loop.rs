use mio::{Events, Interest, Poll, Registry, Token};
use std::io;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::fmt;

/// 可执行事务的trait
pub trait Executor: Send + Sync {
    /// 检查是否就绪
    fn is_ready(&self, interest: Interest) -> bool;
    /// 处理就绪事件
    fn handle_ready(&mut self, interest: Interest) -> io::Result<()>;
    /// 清理资源
    fn cleanup(&mut self) -> io::Result<()>;
}

/// 事件循环管理
pub struct EventLoop {
    registry: Registry,
    events: Events,
    executors: HashMap<Token, Arc<Mutex<dyn Executor>>>,
    next_token: usize,
}

impl EventLoop {
    /// 创建新的事件循环
    pub fn new() -> io::Result<Self> {
        let poll = mio::Poll::new()?;
        let registry = poll.registry().try_clone()?;
        let events = Events::with_capacity(1024);
        
        Ok(EventLoop {
            registry,
            events,
            executors: HashMap::new(),
            next_token: 0,
        })
    }
    

    /// 注册执行器到事件循环
    pub fn register(
        &mut self,
        executor: Arc<Mutex<dyn Executor>>,
        token: Token,
    ) -> io::Result<()> {
        self.executors.insert(token, executor);
        Ok(())
    }

    /// 从事件循环注销执行器
    pub fn deregister(&mut self, token: Token) -> io::Result<()> {
        if let Some(executor) = self.executors.remove(&token) {
            let mut guard = executor.lock().unwrap();
            guard.cleanup()?;
        }
        Ok(())
    }

    /// 轮询事件
    pub fn poll(&mut self, timeout: Option<std::time::Duration>) -> io::Result<&Events> {
        let mut poll = Poll::new()?;
        poll.poll(&mut self.events, timeout)?;
        Ok(&self.events)
    }

    /// 运行事件循环
    pub fn run(&mut self) -> io::Result<()> {
        // 主事件循环，会持续运行直到程序终止
        loop {
            // 1. 阻塞等待事件到来
            // poll(None)会阻塞直到有事件发生
            // 不会因为处理完一个事件就退出
            self.poll(None)?;
            
            // 2. 处理所有就绪事件
            // 可能有多个socket同时有事件发生
            for event in self.events.iter() {
                if let Some(executor) = self.get_executor(event.token()) {
                    let mut guard = executor.lock().unwrap();
                    
                    // 3. 处理可读事件
                    // 即使处理完一个消息，循环仍会继续等待新事件
                    if event.is_readable() {
                        // 这里只是标记事件就绪，实际消息处理在Executor实现中
                        // 可以处理多条消息，直到socket缓冲区为空
                        guard.handle_ready(Interest::READABLE)?;
                    }
                }
            }
            
            // 4. 准备下一轮循环
            // 清空事件集合不会影响socket的持续监听
            // 注册时的监听关系仍然保持
            self.events.clear();
            
            // 5. 循环回到顶部，继续等待新事件
            // 只有程序终止或显式break才会退出
        }
    }

    /// 通过token获取执行器
    pub fn get_executor(&self, token: Token) -> Option<&Arc<Mutex<dyn Executor>>> {
        self.executors.get(&token)
    }
}

impl fmt::Debug for EventLoop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventLoop")
            .field("registry", &self.registry)
            .field("events", &self.events)
            .field("next_token", &self.next_token)
            // 跳过sockets字段的调试输出
            .finish()
    }
}