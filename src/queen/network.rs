//! 纯zmq+Protobuf的网络通信模块

use std::sync::{Arc, Mutex};
use prost::Message;
use thiserror::Error;
use zmq::{Context, Socket, PollItem, POLLIN};

use crate::proto::zergpool::{Heartbeat, Registration, Response, Task};
use crate::RegistrationError;
use crate::{ProcessMessage, proto::zergpool};

/// 网络通信错误类型
#[derive(Error, Debug)]
pub enum NetworkError {
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

        let should_exit = Arc::new(Mutex::new(false));

        Ok(Self {
            zmq_ctx,
            zmq_socket,
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

    /// 轮询网络事件(单次轮询)
    pub fn poll_events(&mut self) -> Result<Vec<(String, ProcessMessage)>, NetworkError> {
        let mut messages = Vec::new();
        let mut poll_items = [self.zmq_socket.as_poll_item(POLLIN)];

        // 单次轮询，超时100ms
        zmq::poll(&mut poll_items, 100)?;

        if poll_items[0].is_readable() {
            println!("[NETWORK TRACE] 检测到可读事件");
            
            // 使用recv_multipart一次性接收所有帧
            let frames = self.zmq_socket.recv_multipart(0)?;
            
            // 验证四帧消息格式
            if frames.len() != 4 {
                log::warn!("消息格式错误：期望4帧，实际收到{}帧", frames.len());
                return Ok(messages);
            }
            
            // 解析身份帧
            let identity = String::from_utf8_lossy(&frames[0]).to_string();
            println!("[NETWORK TRACE] 收到身份帧: {}", identity);
            
            // 验证空帧
            if !frames[1].is_empty() || !frames[2].is_empty() {
                log::warn!("消息格式错误：空帧非空");
                return Ok(messages);
            }
            
            // 处理数据帧
            println!("[NETWORK TRACE] 收到数据帧: {}字节", frames[3].len());
            match self.parse_message(&frames[3]) {
                Ok(msg) => messages.push((identity, msg)),
                Err(e) => log::warn!("消息解析失败: {}", e),
            }
        }
        Ok(messages)
    }

    /// 发送响应消息(优化日志)
    pub fn send_response(&mut self, identity: &str, response: &Response) -> Result<(), NetworkError> {
        let mut buf = Vec::new();
        response.encode(&mut buf)?;
        
        self.zmq_socket.send(identity.as_bytes(), zmq::SNDMORE)?;
        self.zmq_socket.send(&buf, 0)?;
        
        log::debug!("已发送响应给 {} ({}字节)", identity, buf.len());
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
