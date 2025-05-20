//! 工蜂节点心跳机制实现
//! 
//! 严格遵循docs/架构设计.md第206-215行规范

use std::time::{Duration, Instant};
use tokio::time;
use zmq::{Context, Socket, DEALER};
use prost::Message;
use crate::ProcessId;
use crate::proto::zergpool::Heartbeat;

/// 心跳管理器
pub struct HeartbeatManager {
    worker_id: ProcessId,
    zmq_socket: Socket,
    last_recv_time: Instant,
    timeout_count: u32,
}

impl HeartbeatManager {
    /// 创建新的心跳管理器
    pub fn new(worker_id: ProcessId, zmq_endpoint: &str) -> Result<Self, zmq::Error> {
        let ctx = Context::new();
        let socket = ctx.socket(DEALER)?;
        socket.connect(zmq_endpoint)?;

        Ok(Self {
            worker_id,
            zmq_socket: socket,
            last_recv_time: Instant::now(),
            timeout_count: 0,
        })
    }

    /// 启动心跳循环
    pub async fn start(&mut self) -> Result<(), HeartbeatError> {
        let mut interval = time::interval(self.next_interval());
        
        loop {
            interval.tick().await;
            
            // 发送心跳
            self.send_heartbeat().await?;
            
            // 检查超时
            if self.check_timeout() {
                return Err(HeartbeatError::Timeout);
            }
        }
    }

    /// 计算下一个心跳间隔(30秒±5%随机抖动)
    fn next_interval(&self) -> Duration {
        let base = Duration::from_secs(30);
        let jitter = rand::random_range(0..=10);
        base + (base * jitter) / 100 - base / 20
    }

    /// 发送心跳消息
    async fn send_heartbeat(&self) -> Result<(), HeartbeatError> {
        let msg = Heartbeat {
            worker_id: self.worker_id.clone(),
            timestamp: chrono::Utc::now().timestamp(),
        };
        
        let mut buf = Vec::new();
        msg.encode(&mut buf)?;
        self.zmq_socket.send(&buf, 0)?;
        
        Ok(())
    }

    /// 检查超时(5次心跳超时判定离线)
    fn check_timeout(&mut self) -> bool {
        if self.last_recv_time.elapsed() > Duration::from_secs(30) {
            self.timeout_count += 1;
            if self.timeout_count >= 5 {
                return true;
            }
        } else {
            self.timeout_count = 0;
        }
        false
    }
}

/// 心跳错误类型
#[derive(thiserror::Error, Debug)]
pub enum HeartbeatError {
    #[error("ZMQ通信错误")]
    Zmq(#[from] zmq::Error),
    #[error("消息编码错误")]
    Encode(#[from] prost::EncodeError),
    #[error("心跳超时")]
    Timeout,
}