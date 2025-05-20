//! 工蜂(Worker)节点实现模块

pub mod heartbeat;
pub mod network;
pub mod task_queue;

pub use network::get_worker_id;
pub use heartbeat::HeartbeatManager;
pub use task_queue::{TaskQueue, CallbackHandler, set_callback_handler};