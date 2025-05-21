use mio::{Events, Interest, Poll, Token};
use mio::net::{TcpListener, TcpStream};
use std::io::{Read, Write};
use std::thread;
use std::time::Duration;

const SERVER: Token = Token(0);
const CLIENT: Token = Token(1);

#[test]
fn test_basic_mio_communication() {
    let port = 5555;
    
    // 启动服务器线程
    let server_thread = thread::spawn(move || {
        let addr = format!("127.0.0.1:{}", port).parse().unwrap();
        println!("[SERVER] Binding to {}", addr);
        let mut listener = TcpListener::bind(addr).expect("Failed to bind");
        
        let mut poll = Poll::new().unwrap();
        poll.registry()
            .register(&mut listener, SERVER, Interest::READABLE)
            .expect("Failed to register server");
        
        println!("[SERVER] Waiting for connections...");
        let mut events = Events::with_capacity(128);
        poll.poll(&mut events, Some(Duration::from_secs(10))).expect("Poll failed");
        
        for event in events.iter() {
            if event.token() == SERVER {
                println!("[SERVER] Accepting connection...");
                let (mut stream, _) = listener.accept().expect("Accept failed");
                let mut buf = [0; 1024];
                
                // 处理非阻塞读取
                let mut retries = 0;
                let n = loop {
                    match stream.read(&mut buf) {
                        Ok(n) => break n,
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            retries += 1;
                            if retries > 10 {
                                panic!("Max retries exceeded");
                            }
                            thread::sleep(Duration::from_millis(10));
                            continue;
                        }
                        Err(e) => panic!("Read failed: {:?}", e),
                    }
                };
                
                println!("[SERVER] Received: {:?}", &buf[..n]);
                assert_eq!(&buf[..n], b"test message");
                
                // 处理非阻塞写入
                let mut written = 0;
                while written < b"received".len() {
                    match stream.write(&b"received"[written..]) {
                        Ok(n) => written += n,
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(10));
                            continue;
                        }
                        Err(e) => panic!("Write failed: {:?}", e),
                    }
                }
                println!("[SERVER] Response sent");
            }
        }
    });

    // 给服务器足够时间启动
    thread::sleep(Duration::from_millis(500));

    // 客户端测试
    println!("[CLIENT] Connecting to server...");
    let addr: std::net::SocketAddr = format!("127.0.0.1:{}", port).parse().expect("Invalid address");
    let mut stream = TcpStream::connect(addr).expect("Connect failed");
    let mut poll = Poll::new().unwrap();
    poll.registry()
        .register(&mut stream, CLIENT, Interest::WRITABLE | Interest::READABLE)
        .expect("Failed to register client");
    
    println!("[CLIENT] Sending message...");
    // 处理非阻塞发送
    let mut written = 0;
    while written < b"test message".len() {
        match stream.write(&b"test message"[written..]) {
            Ok(n) => written += n,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(10));
                continue;
            }
            Err(e) => panic!("Send failed: {:?}", e),
        }
    }
    
    let mut events = Events::with_capacity(128);
    poll.poll(&mut events, Some(Duration::from_secs(10))).expect("Client poll failed");
    
    for event in events.iter() {
        if event.token() == CLIENT && event.is_readable() {
            println!("[CLIENT] Reading response...");
            let mut buf = [0; 1024];
            
            // 处理非阻塞读取
            let mut retries = 0;
            let n = loop {
                match stream.read(&mut buf) {
                    Ok(n) => break n,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        retries += 1;
                        if retries > 10 {
                            panic!("Max retries exceeded");
                        }
                        thread::sleep(Duration::from_millis(10));
                        continue;
                    }
                    Err(e) => panic!("Read failed: {:?}", e),
                }
            };
            
            println!("[CLIENT] Received: {:?}", &buf[..n]);
            assert_eq!(&buf[..n], b"received");
        }
    }

    server_thread.join().unwrap();
}