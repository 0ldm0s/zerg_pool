use criterion::{black_box, criterion_group, criterion_main, Criterion};
use zerg_pool::Process;
use zerg_pool::balancer::ZergRushSelector;
use std::time::Duration;

fn test_nodes(count: usize) -> Vec<Process> {
    (0..count)
        .map(|i| {
            let mut p = Process::new(
                format!("node-{}", i),
                Vec::new(),
                None
            );
            p.weight = 1.0;
            p.current_load = 0.0;
            p
        })
        .collect()
}

pub fn bench_node_selection(c: &mut Criterion) {
    let nodes = test_nodes(100);
    let selector = ZergRushSelector::new(0.8, Duration::from_secs(5));
    let weighted_nodes = nodes.iter().map(|n| (n, 0.5)).collect::<Vec<_>>();

    c.bench_function("select from 100 nodes", |b| {
        b.iter(|| {
            let _ = selector.select(black_box(&weighted_nodes));
        })
    });
}

criterion_group!{
    name = benches;
    config = Criterion::default();
    targets = bench_node_selection
}
criterion_main!(benches);