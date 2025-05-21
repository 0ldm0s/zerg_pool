//! 动态权重计算模块
//! 
//! 实现基于CPU(60%)、内存(30%)、网络(10%)的加权计算
//! 并集成EMA平滑处理

use moving_averages::ema::Ema;

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