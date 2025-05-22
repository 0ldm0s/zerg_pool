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

    // 创建测试客户端(DEALER套接字)
    let ctx = Context::new();
    let client = ctx.socket(zmq::DEALER).unwrap();
    let identity = "test-drone-1";
    client.set_identity(identity.as_bytes()).unwrap();
    println!("[CLIENT] 正在连接服务端...");
    client.connect("tcp://127.0.0.1:5555").unwrap();

    // 准备注册消息
    let reg = Registration {
        worker_id: identity.to_string(),
        capabilities: vec!["compute".to_string()],
        max_threads: 4,
        version: "1.0.0".to_string(),
    };
    let mut msg_data = Vec::new();
    reg.encode(&mut msg_data).unwrap();

    // 严格按照queen/network.rs要求的四帧格式发送
    // 1. 身份帧(DEALER会自动添加)
    // 2. 空帧1
    // 3. 空帧2 
    // 4. 数据帧
    println!("[CLIENT] 发送四帧格式注册消息...");
    client.send("", zmq::SNDMORE).unwrap(); // 空帧1
    client.send("", zmq::SNDMORE).unwrap(); // 空帧2
    client.send(&msg_data, 0).unwrap(); // 数据帧
    println!("[CLIENT] 消息发送完成 ({}字节)", msg_data.len());

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
    client.disconnect("tcp://127.0.0.1:5555").unwrap();
}