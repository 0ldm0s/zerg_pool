use criterion::{criterion_group, criterion_main, Criterion};
use zerg_pool::ZergPool;
use std::thread;

fn bench_thread_pool(c: &mut Criterion) {
    c.bench_function("thread_pool", |b| {
        let pool = ZergPool::new(4).unwrap();
        b.iter(|| {
            for _ in 0..100 {
                pool.execute(|| {
                    thread::sleep(std::time::Duration::from_micros(10));
                });
            }
        });
    });
}

fn bench_raw_threads(c: &mut Criterion) {
    c.bench_function("raw_threads", |b| {
        b.iter(|| {
            let mut handles = vec![];
            for _ in 0..100 {
                handles.push(thread::spawn(|| {
                    thread::sleep(std::time::Duration::from_micros(10));
                }));
            }
            for handle in handles {
                handle.join().unwrap();
            }
        });
    });
}

criterion_group!(benches, bench_thread_pool, bench_raw_threads);
criterion_main!(benches);