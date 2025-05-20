//! Drone端网络通信模块 - DEALER socket实现

use std::time::{Duration, Instant};
use thiserror::Error;
use zmq::{Context, Socket};
use uuid::Uuid;
use bytes::Bytes;

use crate::proto::zergpool::{Heartbeat, Registration, Response, Task};
use prost::Message;
use std::env;
use std::thread;
use std::sync::OnceLock;

static WORKER_ID: OnceLock<String> = OnceLock::new();

/// 获取当前worker ID
pub fn get_worker_id() -> Option<&'static String> {
    WORKER_ID.get()
}

/// 网络通信错误类型
#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("ZMQ error: {0}")]
    Zmq(#[from] zmq::Error),
    #[error("Protobuf decode error: {0}")]
    Decode(#[from] prost::DecodeError),
    #[error("Protobuf encode error: {0}")]
    Encode(#[from] prost::EncodeError),
}

/// Drone网络连接
pub struct DroneNetwork {
    socket: Socket, // DEALER socket
    last_heartbeat: Instant,
    id: String,     // Worker ID
}

impl DroneNetwork {
    /// 创建新的Drone网络连接
    pub fn connect(queen_addr: &str, port: u16) -> Result<Self, NetworkError> {
        let ctx = Context::new();
        let socket = ctx.socket(zmq::DEALER)?;
        socket.connect(&format!("tcp://{}:{}", queen_addr, port))?;

        let id = Uuid::new_v4().to_string();
        WORKER_ID.set(id.clone()).expect("Worker ID already set");
        
        Ok(Self {
            socket,
            last_heartbeat: Instant::now(),
            id,
        })
    }

    /// 发送注册消息
    pub fn register(&self, worker_id: &str, capabilities: Vec<String>) -> Result<(), NetworkError> {
        let reg = Registration {
            worker_id: worker_id.to_string(),
            max_threads: thread::available_parallelism().map_or(4, |n| n.get() as i32),
            version: env!("CARGO_PKG_VERSION").to_string(),
            capabilities,
        };
        let mut buf = Vec::new();
        reg.encode(&mut buf)?;
        self.socket.send(&buf, 0)?;
        Ok(())
    }

    /// 发送心跳
    pub fn send_heartbeat(&mut self) -> Result<(), NetworkError> {
        if self.last_heartbeat.elapsed() > Duration::from_secs(5) {
            let hb = Heartbeat {
                worker_id: self.id.clone(),
                timestamp: chrono::Utc::now().timestamp(),
            };
            let mut buf = Vec::new();
            hb.encode(&mut buf)?;
            self.socket.send(&buf, 0)?;
            self.last_heartbeat = Instant::now();
        }
        Ok(())
    }

    /// 接收任务
    pub fn recv_task(&self) -> Result<Option<Task>, NetworkError> {
        if let Ok(msg) = self.socket.recv_bytes(0) {
            Ok(Some(Task::decode(&*msg)?))
        } else {
            Ok(None)
        }
    }

    /// 发送任务结果
    pub fn send_response(&self, response: &Response) -> Result<(), NetworkError> {
        let mut buf = Vec::new();
        response.encode(&mut buf)?;
        self.socket.send(&buf, 0)?;
        Ok(())
    }
}