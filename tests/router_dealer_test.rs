use zmq::{Context, SocketType};
use zerg_pool::proto::zergpool::Registration;
use prost::Message;
use thiserror::Error;

#[derive(Error, Debug)]
enum TestError {
    #[error("ZMQ error: {0}")]
    Zmq(#[from] zmq::Error),
    #[error("Encoding error: {0}")]
    Encode(#[from] prost::EncodeError),
    #[error("Decoding error: {0}")]
    Decode(#[from] prost::DecodeError),
    #[error("Thread join error")]
    ThreadJoin,
}

fn send_registration(dealer: &zmq::Socket) -> Result<(), TestError> {
    let mut reg = Registration::default();
    reg.worker_id = "test-drone-1".to_string();
    let mut msg_data = Vec::new();
    reg.encode(&mut msg_data)?;
    
    dealer.send("", zmq::SNDMORE)?;
    dealer.send("", zmq::SNDMORE)?;
    dealer.send(&msg_data, 0)?;
    Ok(())
}

#[test]
fn test_router_dealer_pollin() -> Result<(), TestError> {
    // 服务端ROUTER
    let ctx = Context::new();
    let router = ctx.socket(SocketType::ROUTER)?;
    router.bind("tcp://127.0.0.1:5556")?;

    // 客户端线程
    let client_thread = std::thread::spawn(move || -> Result<(), TestError> {
        let ctx = Context::new();
        let dealer = ctx.socket(SocketType::DEALER)?;
        dealer.set_identity(b"test-client")?;
        dealer.connect("tcp://127.0.0.1:5556")?;

        send_registration(&dealer)?;
        Ok(())
    });

    // 使用ZMQ POLLIN机制
    let mut items = [router.as_poll_item(zmq::POLLIN)];
    zmq::poll(&mut items, 1000)?;

    if items[0].is_readable() {
        println!("[DEBUG] 开始接收消息帧...");
        
        // ROUTER会添加身份帧作为第一帧
        let identity = router.recv_string(0)?.unwrap();
        println!("[DEBUG] 第1帧(身份帧): {:?}", identity);
        assert_eq!(identity, "test-client");
        
        // 检查是否有更多帧
        router.get_rcvmore()?;
        let empty1 = router.recv_bytes(0)?;
        println!("[DEBUG] 第2帧(空帧1): 长度={}", empty1.len());
        assert!(empty1.is_empty());
        
        // 检查是否有更多帧
        router.get_rcvmore()?;
        let empty2 = router.recv_bytes(0)?;
        println!("[DEBUG] 第3帧(空帧2): 长度={}", empty2.len());
        assert!(empty2.is_empty());
        
        // 检查是否有更多帧
        router.get_rcvmore()?;
        let data = router.recv_bytes(0)?;
        println!("[DEBUG] 第4帧(数据帧): 长度={}", data.len());
        
        let decoded = Registration::decode(&data[..])?;
        println!("[DEBUG] 解码结果: {:?}", decoded);
        assert_eq!(decoded.worker_id, "test-drone-1");
    }

    client_thread.join().map_err(|_| TestError::ThreadJoin)??;
    Ok(())
}