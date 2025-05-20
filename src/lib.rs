//! ZergPool 核心库入口 - 严格遵循docs/架构设计.md规范

mod queen;
mod drone;
mod proto;

/// 进程标识类型
pub type ProcessId = String;

/// 进程结构体
#[derive(Debug, Clone)]
pub struct Process {
    pub id: ProcessId,
    pub endpoint: String,
    pub capability: Vec<String>,
}

/// 注册错误类型
#[derive(thiserror::Error, Debug)]
pub enum RegistrationError {
    #[error("无效的终端地址格式")]
    InvalidEndpoint,
    #[error("工作池已满")]
    PoolFull,
}

// 公开导出模块的公共接口
pub use queen::DronePool;
pub use drone::heartbeat::HeartbeatManager;
pub use drone::network::DroneNetwork;
pub use queen::network::HiveNetwork;
