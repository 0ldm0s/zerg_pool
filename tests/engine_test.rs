use zerg_pool::engine::TaskEngine;
use zerg_pool::balancer::ZergRushSelector;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{timeout, Duration};

const TEST_TIMEOUT: Duration = Duration::from_secs(5);

#[tokio::test]
async fn test_basic_task_execution() {
    let balancer = Arc::new(Mutex::new(ZergRushSelector::new(
        0.8,
        std::time::Duration::from_secs(5),
        std::time::Duration::from_secs(10)
    )));
    let mut engine = TaskEngine::new(balancer, 4);
    
    let (tx, mut rx) = mpsc::channel(1);
    
    engine.submit(Box::new(move || {
        tx.blocking_send(42).unwrap();
    })).await;
    
    assert_eq!(rx.recv().await, Some(42));
}

#[tokio::test]
async fn test_graceful_shutdown() {
    let balancer = Arc::new(Mutex::new(ZergRushSelector::new(
        0.8,
        std::time::Duration::from_secs(5),
        std::time::Duration::from_secs(10)
    )));
    let mut engine = TaskEngine::new(balancer, 4);
    
    engine.submit(Box::new(|| {
        std::thread::sleep(std::time::Duration::from_millis(100));
    })).await;
    
    timeout(TEST_TIMEOUT * 2, engine.shutdown())
        .await
        .expect("Shutdown timeout");
}

#[tokio::test]
async fn test_load_balancing_integration() {
    let balancer = Arc::new(Mutex::new(ZergRushSelector::new(
        0.8,
        std::time::Duration::from_secs(5),
        std::time::Duration::from_secs(15)
    )));
    let mut engine = TaskEngine::new(balancer.clone(), 4);
    
    let (tx, mut rx) = mpsc::channel(10);
    
    for i in 0..10 {
        let tx = tx.clone();
        let task = async move {
            let _ = tx.send(i).await;
        };
        engine.submit(Box::new(move || {
            tokio::runtime::Handle::current()
                .block_on(task);
        })).await;
    }
    
    let mut results = Vec::new();
    for _ in 0..10 {
        results.push(rx.recv().await.unwrap());
    }
    
    results.sort();
    assert_eq!(results, (0..10).collect::<Vec<_>>());
}

#[tokio::test]
async fn test_metrics_collection() {
    let balancer = Arc::new(Mutex::new(ZergRushSelector::new(
        0.8,
        std::time::Duration::from_secs(5),
        std::time::Duration::from_secs(10)
    )));
    let mut engine = TaskEngine::new(balancer, 4);

    let (tx, mut rx) = mpsc::channel(1);
    
    engine.submit(Box::new(move || {
        let handle = tokio::runtime::Handle::current();
        handle.block_on(async move {
            let _ = tx.send(1).await;
        });
    })).await;

    assert_eq!(
        timeout(TEST_TIMEOUT, rx.recv())
            .await
            .expect("Metrics collection timeout"),
        Some(1)
    );
}
