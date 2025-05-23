//! 动态权重计算模块
//! 
//! 实现基于CPU(60%)、内存(30%)、网络(10%)的加权计算
//! 并集成EMA平滑处理

use moving_averages::ema::Ema;
use rand::prelude::*;
use thiserror::Error;
use tokio::time::{Duration, sleep};
use metrics::{counter, gauge};
use crate::Process;

/// 权重计算器
#[derive(Debug)]
pub struct WeightCalculator {
    cpu_ema: Ema<f64>,
    mem_ema: Ema<f64>,
    net_ema: Ema<f64>,
}

impl WeightCalculator {
    /// 创建新的权重计算器
    /// 
    /// # 参数
    /// - alpha: EMA平滑系数(0-1)
    pub fn new(alpha: f64) -> Self {
        Self {
            cpu_ema: Ema::new(alpha),
            mem_ema: Ema::new(alpha),
            net_ema: Ema::new(alpha),
        }
    }

    /// 更新指标并计算当前权重
    /// 
    /// # 参数
    /// - cpu: CPU使用率(0.0-1.0)
    /// - mem: 内存使用率(0.0-1.0)
    /// - net: 网络延迟(ms)，值越小越好
    /// 
    /// # 返回
    /// 计算后的权重值(0.0-1.0)
    pub fn calculate(&mut self, cpu: f64, mem: f64, net: f64) -> f64 {
        // 标准化网络延迟(假设最大延迟1000ms)
        let net_score = 1.0 - (net.min(1000.0) / 1000.0);
        
        // 应用EMA平滑
        let cpu_smoothed = self.cpu_ema.next(cpu);
        let mem_smoothed = self.mem_ema.next(mem);
        let net_smoothed = self.net_ema.next(net_score);

        // 加权计算
        0.6 * cpu_smoothed + 0.3 * mem_smoothed + 0.1 * net_smoothed
    }

    /// 重置所有EMA状态
    pub fn reset(&mut self) {
        self.cpu_ema.reset();
        self.mem_ema.reset();
        self.net_ema.reset();
    }
}

/// 备用节点池配置
#[derive(Debug)]
pub struct BackupPool {
    nodes: Vec<Process>,
    warmup_duration: Duration,
}

impl BackupPool {
    /// 创建新的备用节点池
    pub fn new(warmup_duration: Duration) -> Self {
        Self {
            nodes: Vec::new(),
            warmup_duration,
        }
    }

    /// 添加节点到备用池
    pub fn add_node(&mut self, node: Process) {
        self.nodes.push(node);
        counter!("zergpool.backup_nodes").increment(1);
    }

    /// 从备用池取出节点
    pub fn take_node(&mut self) -> Option<Process> {
        let node = self.nodes.pop();
        if node.is_some() {
            counter!("zergpool.backup_nodes").increment(0); // 重置为0
        }
        node
    }

    /// 获取当前备用节点数量
    pub fn len(&self) -> usize {
        self.nodes.len()
    }
}


#[derive(Debug, Error)]
pub enum SelectorError {
    #[error("No available nodes")]
    NoNodesAvailable,
    #[error("Invalid load value")]
    InvalidLoad,
}

#[derive(Debug, Error)]
pub enum ScaleError {
    #[error("No backup nodes available")]
    NoBackupNodes,
    #[error("Migration timeout")]
    MigrationTimeout,
}

/// Zerg Rush算法选择器
#[derive(Debug)]
pub struct ZergRushSelector {
    /// 总任务数统计
    total_tasks: u64,
    /// 总耗时统计(毫秒)
    total_duration: u64,
    /// 最大负载阈值
    max_load_threshold: f64,
    /// 健康检查间隔
    check_interval: Duration,
    /// 备用进程池
    backup_pool: BackupPool,
}

impl ZergRushSelector {
    /// 创建新选择器
    ///
    /// # 参数
    /// - max_load_threshold: 最大负载阈值(0.0-1.0)
    /// - check_interval: 健康检查间隔
    /// - warmup_duration: 节点预热时长
    pub fn new(max_load_threshold: f64, check_interval: Duration, warmup_duration: Duration) -> Self {
        // 初始化健康检查指标
        gauge!("zergpool.healthcheck_interval").set(check_interval.as_secs_f64());
        Self {
            total_tasks: 0,
            total_duration: 0,
            max_load_threshold,
            check_interval,
            backup_pool: BackupPool::new(warmup_duration),
        }
    }

    /// 任务完成回调
    pub fn on_task_completed(&mut self, duration: std::time::Duration) {
        self.total_tasks += 1;
        self.total_duration += duration.as_millis() as u64;
    }

    /// 选择最优节点
    /// 
    /// # 参数
    /// - nodes: 节点列表(包含Process和负载信息)
    /// 
    /// # 返回
    /// 选中的Process或错误
    pub fn select<'a>(
        &self,
        nodes: &'a [(&Process, f64)],
    ) -> Result<&'a Process, SelectorError> {
        // 过滤负载低于阈值的节点
        let mut candidates: Vec<&(&Process, f64)> = nodes
            .iter()
            .filter(|(_, load)| *load <= self.max_load_threshold)
            .collect();

        if candidates.is_empty() {
            return Err(SelectorError::NoNodesAvailable);
        }

        // 按负载升序排序
        candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        // 取前N个节点(N = CPU核心数/2)
        let num_candidates = (num_cpus::get() / 2).max(1);
        candidates.truncate(num_candidates);

        // 随机选择
        let mut rng = rand::rng();
        Ok(candidates.choose(&mut rng).unwrap().0)
    }

    /// 扩容操作
    pub async fn scale_out(&mut self, main_pool: &mut Vec<Process>) -> Result<(), ScaleError> {
        if self.backup_pool.len() == 0 {
            return Err(ScaleError::NoBackupNodes);
        }

        // 从备用池取出节点
        let new_node = self.backup_pool.take_node()
            .ok_or(ScaleError::NoBackupNodes)?;

        // 渐进式权重迁移
        let steps = 10;
        let step_duration = self.backup_pool.warmup_duration / steps as u32;
        
        for i in 1..=steps {
            let weight = i as f64 / steps as f64;
            gauge!("zergpool.migration_weight").set(weight);
            sleep(step_duration).await;
        }

        // 加入主节点池
        main_pool.push(new_node);
        counter!("zergpool.scale_out").increment(1);
        Ok(())
    }

    /// 缩容操作
    pub async fn scale_in(&mut self, main_pool: &mut Vec<Process>) -> Result<(), ScaleError> {
        if main_pool.is_empty() {
            return Err(ScaleError::NoBackupNodes);
        }

        // 选择负载最低的节点
        let node = main_pool.pop().unwrap();
        self.backup_pool.add_node(node);
        counter!("zergpool.scale_in").increment(1);
        Ok(())
    }

    /// 任务提交通知
    ///
    /// 当有新任务提交时调用，用于触发负载均衡决策
    pub fn on_task_submitted(&mut self) {
        counter!("zergpool.tasks_submitted").increment(1);
        // 后续可在此处添加自动扩容逻辑
    }
}