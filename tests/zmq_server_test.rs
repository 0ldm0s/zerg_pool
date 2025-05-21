use zmq::Context;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[test]
fn test_basic_zmq_communication() {
    let port = 5555;
    let port_arc = Arc::new(port);

    // 启动服务器线程
    let server_port = port_arc.clone();
    let server_thread = thread::spawn(move || {
        let ctx = Context::new();
        let socket = ctx.socket(zmq::REP).unwrap();
        socket.bind(&format!("tcp://*:{}", server_port)).unwrap();
        
        let msg = socket.recv_string(0).unwrap().unwrap();
        assert_eq!(msg, "测试消息");
        socket.send("已收到", 0).unwrap();
    });

    // 客户端测试
    let client_ctx = Context::new();
    let client_socket = client_ctx.socket(zmq::REQ).unwrap();
    client_socket.connect(&format!("tcp://127.0.0.1:{}", port)).unwrap();

    client_socket.send("测试消息", 0).unwrap();
    let reply = client_socket.recv_string(0).unwrap().unwrap();
    assert_eq!(reply, "已收到");

    server_thread.join().unwrap();
}