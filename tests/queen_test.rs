use std::sync::{Arc, Mutex};
use zerg_pool::{DronePool, ProcessMessage};
use zerg_pool::proto::zergpool::Registration;
use zmq::{Context, Socket};
use prost::Message;

#[tokio::test]
async fn test_drone_pool_with_queen() {
    println!("[TEST CLIENT] 初始化ZMQ上下文");

    // 创建DronePool服务器
    let pool = Arc::new(Mutex::new(
        DronePool::new("127.0.0.1", 5555).expect("Failed to create DronePool")
    ));
    let pool_clone = Arc::clone(&pool);

    // 在后台线程运行事件轮询
    std::thread::spawn(move || {
        println!("[SERVER POLL] 启动事件轮询线程");
        loop {
            {
                let mut pool = pool_clone.lock().unwrap();
                if let Err(e) = pool.poll_events() {
                    println!("[SERVER POLL] 轮询失败: {:?}", e);
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    });

    // 测试1: 工作节点注册
    {
        let mut pool = pool.lock().unwrap();
        let mut drone = zerg_pool::Process::new(
            "worker1".to_string(),
            vec!["compute".to_string()],
            Some(10)
        );
        drone.weight = 1.0;
        pool.register_drone(drone).expect("注册失败");
        assert_eq!(pool.get_worker_count(), 1);
    }

    // 测试2: 工作节点选择
    {
        let mut pool = pool.lock().unwrap();
        let worker_name = "worker1".to_string();
        
        pool.update_worker_metrics(
            &worker_name,
            0.3,  // cpu
            0.5,  // mem
            100,  // latency
            5     // tasks
        );
        
        let selected = pool.get_optimal_worker();
        assert_eq!(selected, Some("worker1".to_string()));

        // 更新为不健康状态
        pool.update_worker_metrics(
            &worker_name,
            0.95,  // 高CPU
            0.95,  // 高内存
            100,
            5
        );
        
        let unhealthy = pool.get_unhealthy_drones();
        assert_eq!(unhealthy, vec!["worker1".to_string()]);
        assert!(pool.get_optimal_worker().is_none());
    }
}