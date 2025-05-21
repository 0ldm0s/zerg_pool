#[cfg(test)]
mod tests {
    use super::*;
    use moving_averages::Ema;

    #[test]
    fn test_ema_calculation() {
        let alpha = 0.5;
        let mut ema = Ema::new(alpha);
        
        // 测试EMA计算正确性
        assert_eq!(ema.next(1.0), 1.0);
        assert_eq!(ema.next(2.0), 1.5);
        assert_eq!(ema.next(3.0), 2.25);
    }

    #[test]
    fn test_ema_edge_cases() {
        let alpha = 0.5;
        
        // 测试零值处理
        let mut ema_zero = Ema::new(alpha);
        assert_eq!(ema_zero.next(0.0), 0.0);

        // 测试极大值处理（使用可计算范围的值）
        let mut ema_max = Ema::new(alpha);
        let max_val = f64::MAX / 2.0; // 防止后续计算溢出
        let first_output = ema_max.next(max_val);
        assert_eq!(first_output, max_val);
        
        // 测试极大值平滑处理（第二次输入应保持相同值）
        let actual_max = ema_max.next(max_val);
        let relative_diff_max = ((actual_max - max_val) / max_val).abs();
        assert!(relative_diff_max < 1e-15, "极大值稳定性误差超限: {}", relative_diff_max);

        // 测试极小值处理（首次输入）
        let mut ema_min = Ema::new(alpha);
        let min_val = f64::MIN;
        assert_eq!(ema_min.next(min_val), min_val);
        
        // 测试极小值平滑处理（第二次输入应保持相同值）
        let actual_min = ema_min.next(min_val);
        let relative_diff_min = ((actual_min - min_val) / min_val).abs();
        assert!(relative_diff_min < 1e-15, "极小值稳定性误差超限: {}", relative_diff_min);
    }
}