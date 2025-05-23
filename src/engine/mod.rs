use std::sync::Arc;
use tokio::sync::{Mutex, mpsc::{self, Receiver}, Notify};
use tokio::time::{timeout, Duration, Instant};
use log::error;

use crate::balancer::ZergRushSelector;

type Task = Box<dyn FnOnce() + Send + 'static>;

/// 任务执行引擎核心组件
pub struct TaskEngine {
    task_sender: mpsc::Sender<Task>,
    task_receiver: Arc<Mutex<mpsc::Receiver<Task>>>,
    shutdown_notify: Arc<Notify>,
    balancer: Arc<Mutex<ZergRushSelector>>,
}

impl TaskEngine {
    /// 创建新引擎实例
    pub fn new(balancer: Arc<Mutex<ZergRushSelector>>, worker_count: usize) -> Self {
        let (task_sender, task_receiver) = mpsc::channel(1024);
        let shared_receiver = Arc::new(Mutex::new(task_receiver));
        let shutdown_notify = Arc::new(Notify::new());
        
        // 启动worker任务
        for _ in 0..worker_count {
            let receiver = shared_receiver.clone();
            let balancer = balancer.clone();
            let shutdown = shutdown_notify.clone();
            
            tokio::spawn(async move {
                loop {
                    tokio::select! {
                        task = async {
                            let mut guard = receiver.lock().await;
                            guard.recv().await
                        } => {
                            if let Some(task) = task {
                                let start_time = Instant::now();
                                tokio::task::spawn_blocking(task).await.unwrap();
                                
                                if let Ok(mut selector) = timeout(Duration::from_millis(100), balancer.lock()).await {
                                    selector.on_task_completed(start_time.elapsed());
                                }
                            }
                        }
                        _ = shutdown.notified() => {
                            break;
                        }
                    }
                }
            });
        }

        Self {
            task_sender,
            task_receiver: shared_receiver,
            shutdown_notify,
            balancer,
        }
    }

    /// 提交新任务到执行队列
    pub async fn submit(&self, task: Task) {
        if let Err(e) = self.task_sender.send(task).await {
            error!("任务提交失败: {}", e);
        }
        
        if let Ok(mut selector) = timeout(Duration::from_millis(100), self.balancer.lock()).await {
            selector.on_task_submitted();
        }
    }

    /// 优雅关闭引擎
    pub async fn shutdown(self) {
        // 通知所有worker停止
        self.shutdown_notify.notify_waiters();
        
        // 等待任务队列清空
        let shutdown_timeout = Duration::from_secs(5);
        let start_time = Instant::now();
        
        while start_time.elapsed() < shutdown_timeout {
            if self.task_sender.capacity() == self.task_sender.max_capacity() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}