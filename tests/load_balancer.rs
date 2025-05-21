//! 负载均衡器测试模块

use zerg_pool::balancer::WeightCalculator;
use approx::assert_relative_eq;

#[test]
fn test_weight_calculation_boundary() {
    let mut calculator = WeightCalculator::new(0.5);
    
    // 测试边界值
    let min_weight = calculator.calculate(0.0, 0.0, 1000.0);
    assert_relative_eq!(min_weight, 0.0, epsilon = 0.001);

    // 测试最大值需要多次调用使EMA收敛(10次迭代)
    for _ in 0..10 {
        calculator.calculate(1.0, 1.0, 0.0);
    }
    let max_weight = calculator.calculate(1.0, 1.0, 0.0);
    assert_relative_eq!(max_weight, 1.0, epsilon = 0.001);
}

#[test]
fn test_weight_calculation_normal() {
    let mut calculator = WeightCalculator::new(0.5);
    
    // 测试正常值
    let weight = calculator.calculate(0.8, 0.5, 200.0);
    let expected = 0.6 * 0.8 + 0.3 * 0.5 + 0.1 * 0.8; // 网络延迟200ms => 0.8分
    assert_relative_eq!(weight, expected, epsilon = 0.001);
}

#[test]
fn test_weight_calculation_extreme() {
    let mut calculator = WeightCalculator::new(0.5);
    
    // 测试极端情况(高CPU低内存)
    let weight = calculator.calculate(0.9, 0.1, 500.0);
    let expected = 0.6 * 0.9 + 0.3 * 0.1 + 0.1 * 0.5; // 网络延迟500ms => 0.5分
    assert_relative_eq!(weight, expected, epsilon = 0.001);
}

#[test]
fn test_ema_smoothing_effect() {
    let mut calculator = WeightCalculator::new(0.2); // 低alpha值使平滑效果更明显
    
    let w1 = calculator.calculate(0.5, 0.5, 500.0);
    let w2 = calculator.calculate(1.0, 1.0, 0.0);
    
    // 验证EMA平滑效果
    assert!(w2 < 1.0, "EMA平滑应使权重不会立即跳变");
    assert!(w2 > w1, "权重应向新值方向移动");
}