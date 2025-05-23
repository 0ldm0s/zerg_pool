use criterion::{black_box, criterion_group, criterion_main, Criterion};
use zerg_pool::engine::TaskEngine;
use zerg_pool::balancer::ZergRushSelector;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Duration;

fn bench_task_submission(c: &mut Criterion) {
    let balancer = Arc::new(Mutex::new(ZergRushSelector::new(
        0.8, // max_load_threshold
        Duration::from_secs(1), // check_interval
        Duration::from_secs(5) // warmup_duration
    )));
    let mut engine = TaskEngine::new(balancer, 4);

    c.bench_function("task_submission", |b| {
        b.iter(|| {
            engine.submit(black_box(Box::new(|| {})));
        })
    });
}

fn bench_high_throughput(c: &mut Criterion) {
    let balancer = Arc::new(Mutex::new(ZergRushSelector::new(
        0.8, // max_load_threshold
        Duration::from_secs(1), // check_interval
        Duration::from_secs(5) // warmup_duration
    )));
    let mut engine = TaskEngine::new(balancer, 4); // 添加worker_count参数

    c.bench_function("high_throughput", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                engine.submit(black_box(Box::new(|| {})));
            }
        })
    });
}

fn bench_graceful_shutdown(c: &mut Criterion) {
    let balancer = Arc::new(Mutex::new(ZergRushSelector::new(
        0.8, // max_load_threshold
        Duration::from_secs(1), // check_interval
        Duration::from_secs(5) // warmup_duration
    )));
    let mut engine = TaskEngine::new(balancer, 4);

    // 预填充任务
    for _ in 0..1000 {
        engine.submit(Box::new(|| {
            std::thread::sleep(Duration::from_micros(10));
        }));
    }

    c.bench_function("graceful_shutdown", |b| {
        b.iter(|| {
            let balancer = Arc::new(Mutex::new(ZergRushSelector::new(
                0.8,
                Duration::from_secs(1),
                Duration::from_secs(5)
            )));
            let engine = TaskEngine::new(balancer, 4);
            engine.shutdown();
        })
    });
}

criterion_group!(
    benches,
    bench_task_submission,
    bench_high_throughput,
    bench_graceful_shutdown
);
criterion_main!(benches);