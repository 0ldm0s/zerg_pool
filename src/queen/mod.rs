//! Queen模块实现 - 严格遵循docs/架构设计.md规范

use std::collections::HashMap;
use std::time::SystemTime;
use super::RegistrationError;

/// 主工作池最大容量
const MAX_MAIN_POOL_SIZE: usize = 10;

/// 进程池管理结构体
pub struct DronePool {
    workers: Vec<super::Process>,
    backup_drones: Vec<super::Process>,
    status: HashMap<super::ProcessId, WorkerStatus>,
    zmq_endpoint: String,
}

/// 工作节点状态
#[derive(Debug)]
struct WorkerStatus {
    load: u8,
    last_heartbeat: SystemTime,
    capability: Vec<String>,
}

impl DronePool {
    /// 创建新的进程池实例
    pub fn new(zmq_endpoint: String) -> Self {
        Self {
            workers: Vec::new(),
            backup_drones: Vec::new(),
            status: HashMap::new(),
            zmq_endpoint,
        }
    }

    /// 注册新的工作节点
    pub fn register_drone(&mut self, drone: super::Process) -> Result<(), RegistrationError> {
        Self::validate_endpoint(&drone.endpoint)?;

        self.status.insert(drone.id.clone(), WorkerStatus {
            load: 0,
            last_heartbeat: SystemTime::now(),
            capability: drone.capability.clone(),
        });

        if self.workers.len() < MAX_MAIN_POOL_SIZE {
            self.workers.push(drone);
        } else {
            self.backup_drones.push(drone);
        }

        self.update_balancer_strategy();
        Ok(())
    }

    /// 获取当前工作节点数量(测试用)
    pub fn get_worker_count(&self) -> usize {
        self.workers.len()
    }

    /// 验证终端地址格式
    fn validate_endpoint(endpoint: &str) -> Result<(), RegistrationError> {
        if endpoint.split(':').count() != 2 {
            return Err(RegistrationError::InvalidEndpoint);
        }
        Ok(())
    }

    /// 更新负载均衡策略
    fn update_balancer_strategy(&mut self) {
        // 实际实现中这里会调用负载均衡算法
    }
}