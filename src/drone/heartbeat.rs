//! 工蜂节点心跳机制实现
//! 
//! 严格遵循docs/架构设计.md第206-215行规范

use std::time::{Duration, Instant};
use tokio::time;
use zmq::{Context, Socket, DEALER};
use prost::Message;
use sysinfo::{System, SystemExt, CpuExt};
use crate::ProcessId;
use crate::proto::zergpool::{Heartbeat, HealthState};

/// 心跳管理器
pub struct HeartbeatManager {
    worker_id: ProcessId,
    zmq_socket: Socket,
    last_recv_time: Instant,
    last_send_time: Instant,
    timeout_count: u32,
    health_state: HealthState,
    sys: System,
    current_tasks: u32,
    max_tasks: u32,
    last_latency: u32,
}

impl HeartbeatManager {
    /// 创建新的心跳管理器
    pub fn new(worker_id: ProcessId, zmq_endpoint: &str, max_tasks: u32) -> Result<Self, zmq::Error> {
        let ctx = Context::new();
        let socket = ctx.socket(DEALER)?;
        socket.connect(zmq_endpoint)?;

        Ok(Self {
            worker_id,
            zmq_socket: socket,
            last_recv_time: Instant::now(),
            last_send_time: Instant::now(),
            timeout_count: 0,
            health_state: HealthState::Healthy,
            sys: System::new_all(),
            current_tasks: 0,
            max_tasks,
            last_latency: 10,
        })
    }

    /// 启动心跳循环
    pub async fn start(&mut self) -> Result<(), HeartbeatError> {
        let mut interval = time::interval(Duration::from_secs(3)); // 调整为3秒
        
        loop {
            interval.tick().await;
            
            self.send_heartbeat().await?;
            
            if self.check_timeout() {
                return Err(HeartbeatError::Timeout);
            }
        }
    }

    /// 发送心跳消息
    async fn send_heartbeat(&mut self) -> Result<(), HeartbeatError> {
        self.sys.refresh_all();
        let cpu_usage = self.sys.global_cpu_info().cpu_usage() / 100.0;
        let mem_usage = self.sys.used_memory() as f32 / self.sys.total_memory() as f32;

        let msg = Heartbeat {
            worker_id: self.worker_id.clone(),
            timestamp: chrono::Utc::now().timestamp(),
            state: self.health_state as i32,
            cpu_usage,
            mem_usage,
            net_latency: self.last_latency,
            current_tasks: self.current_tasks,
            max_tasks: self.max_tasks,
        };
        
        let mut buf = Vec::new();
        msg.encode(&mut buf)?;
        self.last_send_time = Instant::now();
        self.zmq_socket.send(&buf, 0)?;
        
        Ok(())
    }

    /// 处理心跳响应
    pub fn handle_response(&mut self) {
        let rtt = self.last_send_time.elapsed().as_millis() as u32;
        self.last_latency = rtt;
        self.last_recv_time = Instant::now();
    }

    /// 检查超时(9秒超时，连续3次触发熔断)
    fn check_timeout(&mut self) -> bool {
        if self.last_recv_time.elapsed() > Duration::from_secs(9) { // 调整为9秒
            self.timeout_count += 1;
            if self.timeout_count >= 3 {
                self.health_state = HealthState::CircuitBreaker;
                return true;
            }
            self.health_state = HealthState::Unhealthy;
        } else {
            self.timeout_count = 0;
            self.health_state = HealthState::Healthy;
        }
        false
    }

    /// 更新当前任务数
    pub fn update_task_count(&mut self, current: u32) {
        self.current_tasks = current;
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
    #[error("节点已熔断")]
    CircuitBreaker,
}