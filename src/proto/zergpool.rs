/// 任务消息定义
#[derive(serde::Serialize, serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Task {
    /// 任务唯一ID
    #[prost(string, tag = "1")]
    pub id: ::prost::alloc::string::String,
    /// 任务负载数据
    #[prost(bytes = "vec", tag = "2")]
    pub payload: ::prost::alloc::vec::Vec<u8>,
    /// 任务创建时间戳（保持原位置）
    #[prost(int64, tag = "3")]
    pub timestamp: i64,
    /// 任务元数据
    #[prost(map = "string, string", tag = "4")]
    pub metadata: ::std::collections::HashMap<
        ::prost::alloc::string::String,
        ::prost::alloc::string::String,
    >,
    /// 任务优先级（1-10）
    #[prost(uint32, optional, tag = "5")]
    pub priority: ::core::option::Option<u32>,
}
/// 响应消息定义
#[derive(serde::Serialize, serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Response {
    /// 工作节点ID
    #[prost(string, tag = "1")]
    pub worker_id: ::prost::alloc::string::String,
    #[prost(oneof = "response::Result", tags = "2, 3")]
    pub result: ::core::option::Option<response::Result>,
}
/// Nested message and enum types in `Response`.
pub mod response {
    #[derive(serde::Serialize, serde::Deserialize)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Result {
        /// 成功时的输出
        #[prost(bytes, tag = "2")]
        Output(::prost::alloc::vec::Vec<u8>),
        /// 失败时的错误信息
        #[prost(string, tag = "3")]
        Error(::prost::alloc::string::String),
    }
}
/// 工作节点注册消息
#[derive(serde::Serialize, serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Registration {
    /// 工作节点ID
    #[prost(string, tag = "1")]
    pub worker_id: ::prost::alloc::string::String,
    /// 最大线程数（根据CPU核心数）
    #[prost(int32, tag = "2")]
    pub max_threads: i32,
    /// 节点版本（CARGO_PKG_VERSION）
    #[prost(string, tag = "3")]
    pub version: ::prost::alloc::string::String,
    /// 扩展能力列表
    #[prost(string, repeated, tag = "4")]
    pub capabilities: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
}
/// 心跳消息
#[derive(serde::Serialize, serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Heartbeat {
    /// 工作节点ID
    #[prost(string, tag = "1")]
    pub worker_id: ::prost::alloc::string::String,
    /// 心跳时间戳
    #[prost(int64, tag = "2")]
    pub timestamp: i64,
    /// 健康状态
    #[prost(enumeration = "HealthState", tag = "3")]
    pub state: i32,
    /// CPU使用率(0-1)
    #[prost(float, tag = "4")]
    pub cpu_usage: f32,
    /// 内存使用率(0-1)
    #[prost(float, tag = "5")]
    pub mem_usage: f32,
    /// 网络延迟(ms)
    #[prost(uint32, tag = "6")]
    pub net_latency: u32,
    /// 当前正在处理的任务数
    #[prost(uint32, tag = "7")]
    pub current_tasks: u32,
    /// 节点最大并发任务数
    #[prost(uint32, tag = "8")]
    pub max_tasks: u32,
}
/// 响应状态枚举
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum Status {
    /// 成功
    Success = 0,
    /// 失败
    Failure = 1,
    /// 处理中
    Processing = 2,
    /// 超时
    Timeout = 3,
}
impl Status {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            Status::Success => "SUCCESS",
            Status::Failure => "FAILURE",
            Status::Processing => "PROCESSING",
            Status::Timeout => "TIMEOUT",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "SUCCESS" => Some(Self::Success),
            "FAILURE" => Some(Self::Failure),
            "PROCESSING" => Some(Self::Processing),
            "TIMEOUT" => Some(Self::Timeout),
            _ => None,
        }
    }
}
/// 健康状态枚举
#[derive(serde::Serialize, serde::Deserialize)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
#[repr(i32)]
pub enum HealthState {
    Healthy = 0,
    Unhealthy = 1,
    CircuitBreaker = 2,
}
impl HealthState {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            HealthState::Healthy => "HEALTHY",
            HealthState::Unhealthy => "UNHEALTHY",
            HealthState::CircuitBreaker => "CIRCUIT_BREAKER",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "HEALTHY" => Some(Self::Healthy),
            "UNHEALTHY" => Some(Self::Unhealthy),
            "CIRCUIT_BREAKER" => Some(Self::CircuitBreaker),
            _ => None,
        }
    }
}
