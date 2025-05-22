use std::sync::{Arc, Mutex};
use zerg_pool::{DronePool, ProcessMessage};
use zerg_pool::proto::zergpool::Registration;
use zmq::{Context, Socket};
use prost::Message;

#[test]
fn test_drone_pool_with_zmq() {
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

    // 使用DroneNetwork进行注册(与生产环境完全一致)
    let mut drone_net = zerg_pool::drone::network::DroneNetwork::connect("127.0.0.1", 5555)
        .expect("创建DroneNetwork失败");
    
    println!("[CLIENT] 使用DroneNetwork进行注册...");
    drone_net.register("test-drone-1", vec!["compute".to_string()])
        .expect("注册失败");
    println!("[CLIENT] 注册消息已发送");

    // 验证注册结果
    let mut retries = 10;
    let mut registered = false;
    while retries > 0 && !registered {
        {
            let pool = pool.lock().unwrap();
            if pool.get_worker_count() == 1 {
                registered = true;
                break;
            }
        }
        retries -= 1;
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    assert!(registered, "工作节点注册失败");
}