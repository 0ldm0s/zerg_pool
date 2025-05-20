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

use super::{DronePool, RegistrationError};
use crate::proto::zergpool::{Heartbeat, Registration, Response, Task};

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

        Ok(Self { ctx, socket, poll })
    }

    /// 处理网络事件
    pub fn poll_events(&mut self, pool: &mut DronePool) -> Result<(), NetworkError> {
        let mut events = Events::with_capacity(128);
        self.poll.poll(&mut events, Some(Duration::from_millis(100)))?;

        for event in events.iter() {
            if event.token() == Token(0) {
                let msg = self.socket.recv_bytes(0)?;
                self.handle_message(&msg, pool)?;
            }
        }
        Ok(())
    }

    /// 处理接收到的消息
    fn handle_message(&self, data: &[u8], pool: &mut DronePool) -> Result<(), NetworkError> {
        // 优先处理注册消息
        if let Ok(reg) = Registration::decode(data) {
            let endpoint = self.socket.get_last_endpoint()?.unwrap_or_default();
            let process = Process {
                id: reg.worker_id,
                endpoint,
                capability: reg.capabilities,
            };
            pool.register_drone(process)?;
            return Ok(());
        }

        // 其次处理心跳消息
        if let Ok(heartbeat) = Heartbeat::decode(data) {
            log::debug!("Received heartbeat from {}", heartbeat.worker_id);
            return Ok(());
        }

        // 未知消息类型记录日志
        log::warn!("Received unknown message type ({} bytes)", data.len());
        Ok(())
    }

    /// 发送任务响应
    pub fn send_response(&self, response: &Response) -> Result<(), NetworkError> {
        let mut buf = Vec::new();
        response.encode(&mut buf).map_err(NetworkError::from)?;
        self.socket.send(&buf, 0).map_err(NetworkError::from)?;
        Ok(())
    }
}