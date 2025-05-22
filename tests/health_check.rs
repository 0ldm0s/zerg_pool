use mio::{Events, Interest, Poll, Token};
use zmq::{Context, Socket, SocketType};
use zerg_pool::proto::zergpool::Registration;
use prost::Message;

const SERVER_TOKEN: Token = Token(0);

#[test]
fn test_basic_message_flow() -> Result<(), Box<dyn std::error::Error>> {
    // 创建ZMQ上下文和ROUTER socket
    let ctx = Context::new();
    let router = ctx.socket(SocketType::ROUTER)?;
    router.bind("tcp://127.0.0.1:5555")?;

    // 创建mio的Poll实例
    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(128);

    // 手动轮询模式（因Windows下直接集成限制）
    let client_thread = std::thread::spawn(|| {
        let ctx = Context::new();
        let client = ctx.socket(SocketType::DEALER).unwrap();
        client.connect("tcp://127.0.0.1:5555").unwrap();
        
        client.send("Hello from client", 0).unwrap();
        println!("[Client] Sent request");

        let msg = client.recv_string(0).unwrap().unwrap();
        println!("[Client] Received: {}", msg);
    });

    // 手动事件循环（替代mio直接监听）
    loop {
        // 使用ZMQ自带的poll机制检查可读事件
        let mut items = [router.as_poll_item(zmq::POLLIN)];
        zmq::poll(&mut items, 1000).unwrap(); // 超时1秒

        if items[0].is_readable() {
            // 处理ROUTER消息
            let identity = router.recv_msg(0)?;
            let content = router.recv_string(0)?.unwrap();
            println!("[Server] Received: {} from {:?}", content, identity);

            router.send(identity, zmq::SNDMORE)?; // 直接传递Message所有权
            router.send("Response from server", 0)?;
        }

        if client_thread.is_finished() {
            break;
        }
    }

    client_thread.join().unwrap();
    Ok(())
}