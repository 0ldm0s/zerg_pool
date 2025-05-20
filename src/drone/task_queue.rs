//! Drone任务队列模块 - 基于crossbeam-channel实现

use crossbeam_channel::{bounded, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};
use uuid::Uuid;
use crate::proto::zergpool::{Task, Response, response};
use crate::drone::network::NetworkError;
use log::warn;

/// 任务队列配置
const QUEUE_CAPACITY: usize = 1000;
const TIMEOUT_THRESHOLD: Duration = Duration::from_millis(50);

/// 任务队列结构体
pub struct TaskQueue {
    sender: Sender<Task>,
    receiver: Receiver<Response>,
}

impl TaskQueue {
    /// 创建新任务队列
    pub fn new() -> Self {
        let (task_sender, task_receiver) = bounded::<crate::proto::zergpool::Task>(QUEUE_CAPACITY);
        let (resp_sender, resp_receiver) = bounded(QUEUE_CAPACITY);

        // 创建工作线程池
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_cpus::get())
            .build()
            .unwrap();

        // 任务分发线程
        thread::spawn(move || {
            while let Ok(task) = task_receiver.recv() {
                let start_time = Instant::now();
                let resp_sender = resp_sender.clone();
                
                pool.spawn(move || {
                    // 执行任务并生成响应
                    let response = Response {
                        worker_id: crate::drone::get_worker_id()
                            .map(|id| id.as_str())
                            .unwrap_or("unknown")
                            .to_string(),
                        result: Some(crate::proto::zergpool::response::Result::Output(Vec::new())),
                    };
                    
                    // 发送结果
                    if let Err(e) = resp_sender.send(response) {
                        log::error!("Failed to send task result: {}", e);
                    }
                });

                // 检查分发延迟
                if start_time.elapsed() > TIMEOUT_THRESHOLD {
                    log::warn!("Task {} dispatch exceeded P99 latency", task.id);
                }
            }
        });

        // 克隆接收器用于线程
        let resp_receiver_thread = resp_receiver.clone();

        // 启动结果处理线程
        thread::spawn(move || {
            while let Ok(resp) = resp_receiver_thread.recv() {
                // 安全处理回调逻辑
                if let Some(handler) = CALLBACK_HANDLER.get() {
                    let worker_id = resp.worker_id.clone();
                    match handler.handle(&resp) {
                        Ok(_) => (),
                        Err(e) => log::error!("Failed to handle response from worker {}: {}", worker_id, e),
                    }
                }
            }
        });

        Self {
            sender: task_sender,
            receiver: resp_receiver,
        }
    }

    /// 提交新任务
    pub fn submit(&self, task: Task) -> Result<(), NetworkError> {
        self.sender.send(task).map_err(|_| NetworkError::Zmq(zmq::Error::EAGAIN))
    }

    /// 获取结果接收器
    pub fn response_receiver(&self) -> &Receiver<Response> {
        &self.receiver
    }

    /// 获取任务ID
    pub fn generate_task_id() -> String {
        Uuid::new_v4().to_string()
    }
}

use std::fmt;

/// 回调处理器trait
pub trait CallbackHandler: Send + Sync + fmt::Debug {
    fn handle(&self, response: &Response) -> Result<(), NetworkError>;
}

use std::sync::OnceLock;

static CALLBACK_HANDLER: OnceLock<Box<dyn CallbackHandler>> = OnceLock::new();

/// 设置全局回调处理器
pub fn set_callback_handler(handler: Box<dyn CallbackHandler>) {
    CALLBACK_HANDLER.set(handler).expect("Global handler already set");
}

/// 获取全局回调处理器
pub fn get_callback_handler() -> &'static dyn CallbackHandler {
    &**CALLBACK_HANDLER.get().expect("Global handler not set")
}