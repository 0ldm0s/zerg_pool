//! Queen模块实现 - 严格遵循docs/架构设计.md规范

pub mod network;

use std::collections::HashMap;
use std::time::{Instant, Duration};
use super::Process;
use crate::proto::zergpool::HealthState;

/// 主工作池最大容量
const MAX_MAIN_POOL_SIZE: usize = 10;

/// 进程池管理结构体
use std::sync::{Arc, Mutex};

#[derive(Debug)]
struct PoolState {
    workers: Vec<super::Process>,
    backup_drones: Vec<super::Process>,
    status: HashMap<super::ProcessId, WorkerStatus>,
}

impl PoolState {
    fn new() -> Self {
        Self {
            workers: Vec::new(),
            backup_drones: Vec::new(),
            status: HashMap::new(),
        }
    }
}

pub struct DronePool {
    state: Arc<Mutex<PoolState>>,
    network: network::HiveNetwork,
}

/// 工作节点状态(包含外部可访问的指标数据)
#[derive(Debug, Clone)]
pub struct WorkerStatus {
    pub cpu_usage: f32,        // CPU使用率(0.0-1.0)
    pub mem_usage: f32,        // 内存使用率(0.0-1.0)
    pub net_latency: u32,      // 网络延迟(ms)
    pub current_tasks: u32,    // 当前任务数
    pub max_tasks: u32,        // 最大任务数
    pub health_state: HealthState, // 健康状态(与drone端一致)
    
    // 内部管理字段
    last_heartbeat: Instant,
    capability: Vec<String>,
    timeout_count: u32,    // 超时计数(用于熔断)
}

impl DronePool {
    /// 创建新的进程池实例
    pub fn new(bind_addr: &str, port: u16) -> Result<Self, network::NetworkError> {
        println!("[DRONE POOL] 初始化网络层...");
        let network = network::HiveNetwork::new(bind_addr, port)?;
        let full_addr = format!("{}:{}", bind_addr, port);
        println!("[DRONE POOL] 网络初始化完成: {}", full_addr);
        
        Ok(Self {
            state: Arc::new(Mutex::new(PoolState::new())),
            network,
        })
    }

    /// 辅助方法：获取状态锁
    fn with_state<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut PoolState) -> R,
    {
        let mut state = self.state.lock().unwrap();
        f(&mut state)
    }

    /// 辅助方法：获取可变状态锁
    fn with_state_mut<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut PoolState) -> R,
    {
        let mut state = self.state.lock().unwrap();
        f(&mut state)
    }

    /// 注册新的工作节点
    pub fn register_drone(&mut self, drone: super::Process) -> Result<(), network::NetworkError> {
        println!("[REGISTER DRONE] 注册新工作节点: {}", drone.id);
        let need_update = self.with_state_mut(|state| {
            state.status.insert(drone.id.clone(), WorkerStatus {
                last_heartbeat: Instant::now(),
                capability: drone.capability.clone(),
                cpu_usage: 0.0,
                mem_usage: 0.0,
                net_latency: 10,  // 默认10ms
                current_tasks: 0,
                max_tasks: drone.max_tasks.unwrap_or(10),
                health_state: HealthState::Healthy,
                timeout_count: 0,
            });

            if state.workers.len() < MAX_MAIN_POOL_SIZE {
                state.workers.push(drone);
                true
            } else {
                state.backup_drones.push(drone);
                false
            }
        });

        if need_update {
            self.update_balancer_strategy();
        }
        Ok(())
    }

    /// 获取当前工作节点数量(测试用)
    pub fn get_worker_count(&self) -> usize {
        self.with_state(|state| state.workers.len())
    }

    /// 更新负载均衡策略
    fn update_balancer_strategy(&mut self) {
        // 实际实现中这里会调用负载均衡算法
    }
    
    /// 更新节点状态指标(与drone端心跳消息对齐)
    pub fn update_worker_metrics(
        &mut self,
        drone_id: &super::ProcessId,
        cpu_usage: f32,
        mem_usage: f32,
        net_latency: u32,
        current_tasks: u32,
    ) {
        self.with_state_mut(|state| {
            if let Some(status) = state.status.get_mut(drone_id) {
                status.cpu_usage = cpu_usage;
                status.mem_usage = mem_usage;
                status.net_latency = net_latency;
                status.current_tasks = current_tasks;
                status.last_heartbeat = Instant::now();

                // 健康判断逻辑与drone端保持一致
                if status.last_heartbeat.elapsed() > Duration::from_secs(9) {
                    status.timeout_count += 1;
                    if status.timeout_count >= 3 {
                        status.health_state = HealthState::CircuitBreaker;
                    } else {
                        status.health_state = HealthState::Unhealthy;
                    }
                } else {
                    status.timeout_count = 0;
                    let is_overloaded = cpu_usage > 0.9 || mem_usage > 0.9 ||
                                      current_tasks >= status.max_tasks;
                    status.health_state = if is_overloaded {
                        HealthState::Unhealthy
                    } else {
                        HealthState::Healthy
                    };
                }
            }
        })
    }

    /// 获取最优工作节点(基于综合评分)
    pub fn get_optimal_worker(&self) -> Option<super::ProcessId> {
        self.with_state(|state| {
            state.status.iter()
                .filter(|(_, status)| status.health_state == HealthState::Healthy)
                .min_by(|(_, a), (_, b)| {
                    // 评分算法(CPU 40%, 内存 30%, 延迟 20%, 任务负载 10%)
                    let a_score = 0.4 * a.cpu_usage + 0.3 * a.mem_usage +
                                 0.2 * (a.net_latency as f32 / 1000.0) +
                                 0.1 * (a.current_tasks as f32 / a.max_tasks as f32);
                    let b_score = 0.4 * b.cpu_usage + 0.3 * b.mem_usage +
                                 0.2 * (b.net_latency as f32 / 1000.0) +
                                 0.1 * (b.current_tasks as f32 / b.max_tasks as f32);
                    a_score.partial_cmp(&b_score).unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(id, _)| id.clone())
        })
    }

    /// 获取所有不健康节点ID列表
    pub fn get_unhealthy_drones(&self) -> Vec<super::ProcessId> {
        self.with_state(|state| {
            state.status.iter()
                .filter(|(_, status)| {
                    status.health_state != HealthState::Healthy
                })
                .map(|(id, _)| id.clone())
                .collect()
        })
    }

    /// 获取工作节点指标数据(返回副本避免生命周期问题)
    pub fn get_worker_metrics(&self, drone_id: &super::ProcessId) -> Option<WorkerStatus> {
        self.with_state(|state| state.status.get(drone_id).cloned())
    }

    /// 轮询并处理网络事件
    pub fn poll_events(&mut self) -> Result<(), network::NetworkError> {
        println!("[POLL EVENTS] 开始轮询网络事件...");
        let messages = self.network.poll_events()?;
        println!("[POLL EVENTS] 收到 {} 条消息", messages.len());
        
        for (identity, message) in messages {
            println!("[POLL EVENTS] 处理消息 - 来源: {}, 类型: {:?}", identity, message);
            log::debug!("收到消息 - 来源: {}, 类型: {:?}", identity, message);
            match message {
                crate::ProcessMessage::Registration(reg) => {
                    println!("[REGISTRATION] 处理注册消息: {:?}", reg);
                    let mut process = Process::new(
                        reg.worker_id.clone(),
                        reg.capabilities.clone(),
                        Some(reg.max_threads as u32)
                    );
                    process.weight = 1.0;  // 设置默认权重
                    process.current_load = 0.0;  // 初始化负载
                    self.register_drone(process)?;
                    println!("[REGISTRATION] 已注册工作节点: {}", reg.worker_id);
                }
                crate::ProcessMessage::Heartbeat(hb) => {
                    self.update_worker_metrics(
                        &hb.worker_id,
                        hb.cpu_usage,
                        hb.mem_usage,
                        hb.net_latency,
                        hb.current_tasks,
                    );
                    
                    let unhealthy = self.get_unhealthy_drones();
                    if !unhealthy.is_empty() {
                        log::warn!("发现不健康节点: {:?}", unhealthy);
                    }
                }
                _ => {} // 忽略其他消息类型
            }
        }
        Ok(())
    }
}
