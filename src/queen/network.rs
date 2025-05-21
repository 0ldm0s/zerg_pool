//! 精简版网络通信模块 - 仅实现Queen端ROUTER socket

use std::io;
use std::os::windows::io::FromRawSocket;
use std::time::Duration;
use mio::{Events, Interest, Poll, Token};
use mio::net::TcpStream;
use prost::Message;
use thiserror::Error;
use zmq::{Context, Socket};
use crate::Process;

use crate::proto::zergpool::{Heartbeat, Registration, Response, Task};
use crate::RegistrationError;

/// 进程间消息枚举
#[derive(Debug)]
pub enum ProcessMessage {
    Registration(Registration),
    Heartbeat(Heartbeat),
}

/// 网络通信错误类型
#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("ZMQ error: {0}")]
    Zmq(#[from] zmq::Error),
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("Protobuf decode error: {0}")]
    Decode(#[from] prost::DecodeError),
    #[error("Protobuf encode error: {0}")]
    Encode(#[from] prost::EncodeError),
    #[error("Registration error: {0}")]
    Registration(#[from] RegistrationError),
}

/// 网络通信核心结构体
pub struct HiveNetwork {
    ctx: Context,
    socket: Socket, // ROUTER socket
    poll: Poll,
}

impl HiveNetwork {
    /// 获取当前连接的endpoint
    pub fn current_endpoint(&self) -> Result<String, NetworkError> {
        self.socket.get_last_endpoint()
            .map(|opt| opt.unwrap_or_default())
            .map_err(NetworkError::from)
    }

    /// 解析原始消息为结构化数据
    fn parse_message(&self, data: &[u8]) -> Result<ProcessMessage, NetworkError> {
        if let Ok(reg) = Registration::decode(data) {
            Ok(ProcessMessage::Registration(reg))
        } else if let Ok(hb) = Heartbeat::decode(data) {
            Ok(ProcessMessage::Heartbeat(hb))
        } else {
            log::warn!("Received unknown message type ({} bytes)", data.len());
            Err(NetworkError::Decode(prost::DecodeError::new("Unknown message type")))
        }
    }

    /// 创建新的网络实例
    pub fn new(bind_addr: &str, port: u16) -> Result<Self, NetworkError> {
        let ctx = Context::new();
        let socket = ctx.socket(zmq::ROUTER)?;
        socket.bind(&format!("tcp://{}:{}", bind_addr, port))?;

        // 初始化mio事件循环
        let poll = Poll::new()?;
        let zmq_fd = socket.get_fd()?;
        poll.registry().register(
            &mut unsafe { TcpStream::from_raw_socket(zmq_fd) },
            Token(0),
            Interest::READABLE,
        )?;

        Ok(Self {
            ctx,
            socket,
            poll,
        })
    }

    /// 轮询网络事件
    /// 返回解析后的消息列表，由调用方处理业务逻辑
    pub fn poll_events(&mut self) -> Result<Vec<ProcessMessage>, NetworkError> {
        let mut events = Events::with_capacity(128);
        self.poll.poll(&mut events, Some(Duration::from_millis(100)))?;
        let mut messages = Vec::new();

        for event in events.iter() {
            if event.token() == Token(0) {
                let msg = self.socket.recv_bytes(0)?;
                messages.push(self.parse_message(&msg)?);
            }
        }
        Ok(messages)
    }
    /// 发送任务响应
    pub fn send_response(&self, response: &Response) -> Result<(), NetworkError> {
        let mut buf = Vec::new();
        response.encode(&mut buf).map_err(NetworkError::from)?;
        self.socket.send(&buf, 0).map_err(NetworkError::from)?;
        Ok(())
    }
}

