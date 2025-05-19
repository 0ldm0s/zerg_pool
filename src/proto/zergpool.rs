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
    /// 任务创建时间戳
    #[prost(int64, tag = "3")]
    pub timestamp: i64,
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
    /// 支持的能力列表
    #[prost(string, repeated, tag = "2")]
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
}
