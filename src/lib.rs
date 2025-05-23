//! ZergPool 核心库入口 - 严格遵循docs/架构设计.md规范

pub mod balancer;
pub mod drone;
pub mod engine;
pub mod proto;
pub mod queen;

/// 进程标识类型
pub type ProcessId = String;

/// 进程结构体
#[derive(Debug, Clone)]
pub struct Process {
    pub id: ProcessId,
    pub capability: Vec<String>,
    pub max_tasks: Option<u32>, // 可选的最大任务数
    pub weight: f64,
    pub current_load: f64,
}

impl Process {
    /// 创建新进程实例
    pub fn new(id: ProcessId, capability: Vec<String>, max_tasks: Option<u32>) -> Self {
        Self {
            id,
            capability,
            max_tasks,
            weight: 1.0,
            current_load: 0.0,
        }
    }
}

/// 进程间通信消息类型(严格匹配proto/task.proto定义)
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum ProcessMessage {
    /// 工作节点注册消息(对应proto Registration消息)
    Registration(proto::zergpool::Registration),
    
    /// 心跳消息(对应proto Heartbeat消息)
    Heartbeat(proto::zergpool::Heartbeat),
    
    /// 任务消息(对应proto Task消息)
    Task(proto::zergpool::Task),
    
    /// 任务响应消息(对应proto Response消息)
    TaskResponse(proto::zergpool::Response),
}

use crate::queen::network::NetworkError;

/// 通用错误类型
#[derive(thiserror::Error, Debug)]
pub enum RegistrationError {
    #[error("无效的终端地址格式")]
    InvalidEndpoint,
    #[error("工作池已满")]
    PoolFull,
}

#[derive(thiserror::Error, Debug)]
pub enum PoolError {
    #[error("网络通信错误: {0}")]
    Network(#[from] NetworkError),
    
    #[error("工作节点注册失败: {0}")]
    Registration(String),
    
    #[error("无效的工作节点ID")]
    InvalidWorkerId,
    
    #[error("资源不足")]
    InsufficientCapacity,
    
    #[error("内部系统错误")]
    InternalError,
}

pub type Result<T> = std::result::Result<T, PoolError>;

// 公开导出模块的公共接口
pub use queen::DronePool;
pub use drone::heartbeat::HeartbeatManager;
pub use drone::network::DroneNetwork;
pub use engine::TaskEngine;
pub use queen::network::HiveNetwork;


