//! 集成mio+zmq+Protobuf的网络通信模块

use std::io;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use mio::{Events, Interest, Poll, Token};
use prost::Message;
use thiserror::Error;
use zmq::{Context, Socket};

use crate::proto::zergpool::{Heartbeat, Registration, Response, Task};
use crate::RegistrationError;

/// 进程间消息枚举
use crate::{ProcessMessage, proto::zergpool};

/// 网络通信错误类型
#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("Protobuf decode error: {0}")]
    Decode(#[from] prost::DecodeError),
    #[error("Protobuf encode error: {0}")]
    Encode(#[from] prost::EncodeError),
    #[error("Registration error: {0}")]
    Registration(#[from] RegistrationError),
    #[error("ZMQ error: {0}")]
    Zmq(#[from] zmq::Error),
}

/// 网络通信核心结构体
pub struct HiveNetwork {
    zmq_ctx: Context,
    zmq_socket: Socket,
    poll: Poll,
    should_exit: Arc<Mutex<bool>>,
}

impl HiveNetwork {
    /// 创建新的网络实例
    pub fn new(bind_addr: &str, port: u16) -> Result<Self, NetworkError> {
        let full_addr = format!("{}:{}", bind_addr, port);
        Self::bind(&full_addr)
    }

    /// 内部绑定方法(关联函数)
    fn bind(bind_addr: &str) -> Result<Self, NetworkError> {
        let zmq_ctx = Context::new();
        let zmq_socket = zmq_ctx.socket(zmq::ROUTER)?;
        
        let full_addr = format!("tcp://{}", bind_addr);
        println!("[NETWORK] 尝试绑定到 {}", full_addr);
        zmq_socket.bind(&full_addr)?;
        println!("[NETWORK] 成功绑定到 {}", full_addr);

        let poll = Poll::new()?;
        let should_exit = Arc::new(Mutex::new(false));

        // 获取zmq的socket描述符并注册到mio
        let zmq_fd = zmq_socket.get_fd()?;
        println!("[NETWORK DEBUG] ZMQ文件描述符: {}", zmq_fd);
        println!("[NETWORK DEBUG] ZMQ套接字类型: ROUTER");
        println!("[NETWORK DEBUG] ZMQ套接字状态: {:?}", zmq_socket.get_socket_state());
        println!("[NETWORK DEBUG] mio轮询器初始化完成");
        
        #[cfg(windows)] {
            use std::os::windows::io::FromRawSocket;
            use std::net::TcpStream;
            let std_socket = unsafe { TcpStream::from_raw_socket(zmq_fd as _) };
            let mut mio_socket = mio::net::TcpStream::from_std(std_socket);
            poll.registry().register(
                &mut mio_socket,
                Token(0),
                Interest::READABLE,
            )?;
        }

        #[cfg(unix)] {
            use std::os::unix::io::FromRawFd;
            let mut mio_socket = unsafe { mio::net::TcpStream::from_raw_fd(zmq_fd as _) };
            poll.registry().register(
                &mut mio_socket,
                Token(0),
                Interest::READABLE,
            )?;
        }

        Ok(Self {
            zmq_ctx,
            zmq_socket,
            poll,
            should_exit,
        })
    }

    /// 获取当前ZMQ绑定地址
    pub fn current_endpoint(&self) -> Result<String, NetworkError> {
        let result = self.zmq_socket.get_last_endpoint();
        match result {
            Ok(Ok(endpoint)) => Ok(endpoint),
            Ok(Err(e)) => {
                let err_msg = String::from_utf8_lossy(&e);
                log::error!("ZMQ endpoint error: {}", err_msg);
                // 使用zmq内置的EINVAL错误
                Err(NetworkError::Zmq(zmq::Error::EINVAL))
            },
            Err(e) => {
                log::error!("ZMQ socket error: {}", e);
                Err(NetworkError::Zmq(e))
            }
        }
    }
    /// 解析原始消息为结构化数据
    fn parse_message(&self, data: &[u8]) -> Result<ProcessMessage, NetworkError> {
        if let Ok(reg) = Registration::decode(data) {
            Ok(ProcessMessage::Registration(reg))
        } else if let Ok(hb) = Heartbeat::decode(data) {
            Ok(ProcessMessage::Heartbeat(hb))
        } else if let Ok(task) = Task::decode(data) {
            Ok(ProcessMessage::Task(task))
        } else {
            Err(NetworkError::Decode(prost::DecodeError::new("Unknown message type")))
        }
    }

    /// 轮询网络事件
    pub fn poll_events(&mut self) -> Result<Vec<(String, ProcessMessage)>, NetworkError> {
        println!("[NETWORK POLL] 开始轮询ZMQ事件...");
        println!("[NETWORK DEBUG] 当前ZMQ套接字状态: {:?}", self.zmq_socket.get_socket_state());
        let mut events = Events::with_capacity(128);
        self.poll.poll(&mut events, Some(Duration::from_millis(100)))?;
        println!("[NETWORK POLL] 检测到 {} 个事件", events.iter().count());
        println!("[NETWORK DEBUG] mio注册状态: 轮询完成");
        println!("[NETWORK DEBUG] 当前ZMQ套接字状态: {:?}", self.zmq_socket.get_socket_state());
        let mut messages = Vec::new();

        for event in events.iter() {
            if event.is_readable() {
                println!("[NETWORK POLL] 处理可读事件");
                let identity = match self.zmq_socket.recv_string(0) {
                    Ok(Ok(s)) => {
                        println!("[NETWORK RECV] 收到身份帧: {}", s);
                        s
                    },
                    Ok(Err(e)) => {
                        println!("[NETWORK ERROR] 身份帧接收错误: {:?}", e);
                        continue
                    },
                    Err(e) => {
                        println!("[NETWORK ERROR] ZMQ接收错误: {}", e);
                        continue
                    },
                };
                println!("[NETWORK DEBUG] 准备接收消息体...");
                let msg_data = self.zmq_socket.recv_bytes(0)?;
                println!("[NETWORK DEBUG] 实际接收字节数: {}", msg_data.len());
                println!("[NETWORK RECV] 收到消息 - 来源: {}, 长度: {} 字节", identity, msg_data.len());
                
                match self.parse_message(&msg_data) {
                    Ok(msg) => {
                        println!("[NETWORK] 成功解析消息: {:?}", msg);
                        messages.push((identity, msg))
                    },
                    Err(e) => {
                        println!("[NETWORK] 消息解析失败: {}", e);
                        log::warn!("Failed to parse message: {}", e)
                    },
                }
            }
        }

        Ok(messages)
    }

    /// 发送响应消息
    pub fn send_response(&mut self, identity: &str, response: &Response) -> Result<(), NetworkError> {
        println!("[NETWORK SEND] 准备发送响应给: {}", identity);
        let mut buf = Vec::new();
        response.encode(&mut buf)?;
        
        println!("[NETWORK SEND] 发送身份帧: {} 字节", identity.as_bytes().len());
        self.zmq_socket.send(identity.as_bytes(), zmq::SNDMORE)?;
        println!("[NETWORK SEND] 发送消息体: {} 字节", buf.len());
        self.zmq_socket.send(&buf, 0)?;
        println!("[NETWORK SEND] 消息发送完成");
        
        Ok(())
    }

    /// 安全关闭网络连接
    pub fn shutdown(&mut self) {
        *self.should_exit.lock().unwrap() = true;
        if let Ok(endpoint) = self.current_endpoint() {
            let _ = self.zmq_socket.disconnect(&endpoint);
        }
    }
}
