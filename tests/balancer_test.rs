//! 负载均衡器集成测试
//! 
//! 包含三类测试场景：
//! 1. 基础权重计算测试
//! 2. Zerg Rush节点选择测试  
//! 3. 动态扩缩容测试

use proptest::prelude::*;
use criterion::{black_box, Criterion};
use tokio::runtime::Runtime;
use zerg_pool::balancer::{WeightCalculator, ZergRushSelector};
use zerg_pool::Process;
mod test_utils;
use test_utils::TestSelectorBuilder;
use std::time::Duration;

mod scenarios {
    use super::*;

    #[tokio::test]
    async fn test_scale_out_under_high_load() {
        let selector = TestSelectorBuilder::new(80.0, Duration::from_secs(1))
            .with_backup_nodes(vec![test_utils::new_test_process("backup1".into())])
            .build();
        let mut main_pool = vec![test_utils::new_test_process("node1".into())];
        
        // 模拟高负载场景(此时main_pool只有node1)
        let mut selector = selector.await;
        let result = selector.scale_out(&mut main_pool).await;
        assert!(result.is_ok());
        assert_eq!(main_pool.len(), 2);
    }
    
    #[test]
    fn test_zerg_rush_selection() {
        let selector = ZergRushSelector::new(80.0, Duration::from_secs(1), Duration::from_secs(5));
        let nodes = vec![
            test_utils::new_test_process("node1".into()),
            test_utils::new_test_process("node2".into()),
            test_utils::new_test_process("node3".into()),
        ];
        let weighted_nodes: Vec<_> = nodes.iter()
            .zip([60.0, 90.0, 50.0].iter())
            .map(|(p, &w)| (p, w))
            .collect();
        
        let result = selector.select(&weighted_nodes[..]);
        assert!(result.is_ok());
        let selected = result.unwrap();
        assert!(selected.id == "node1" || selected.id == "node3");
    }
}

proptest! {
    #[test]
    fn test_weight_calculation(cpu in 0.0..1.0, mem in 0.0..1.0, net in 0.0..1000.0) {
        let mut calculator = WeightCalculator::new(0.5);
        let weight = calculator.calculate(cpu, mem, net);
        
        // 验证权重在合理范围内
        prop_assert!(weight >= 0.0 && weight <= 1.0);
        
        // 验证权重计算公式
        let expected = 0.6 * cpu + 0.3 * mem + 0.1 * (1.0 - net.min(1000.0)/1000.0);
        prop_assert!((weight - expected).abs() < 0.1); // 允许EMA平滑带来的误差
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut calculator = WeightCalculator::new(0.5);
    
    c.bench_function("weight_calculation", |b| {
        b.iter(|| {
            black_box(calculator.calculate(0.6, 0.3, 100.0));
        })
    });
    
    let selector = ZergRushSelector::new(80.0, Duration::from_secs(1), Duration::from_secs(5));
    let node1 = test_utils::new_test_process("node1".into());
    let node2 = test_utils::new_test_process("node2".into());
    let nodes = &[
        (&node1, 60.0),
        (&node2, 50.0)
    ];
    
    c.bench_function("node_selection", |b| {
        b.iter(|| {
            black_box(selector.select(nodes).unwrap());
        })
    });
}