use mio::{Events, Interest, Poll, Token};
use std::io;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use zmq::{Context, Socket};

const MAX_RETRIES: u32 = 5;
const BASE_DELAY_MS: u64 = 50;
const TEST_MESSAGE: &str = "PING";

struct TestResources {
    ctx: Context,
    server: Socket,
    should_exit: Arc<Mutex<bool>>,
}

fn setup_mio_with_zmq() -> io::Result<()> {
    let ctx = Context::new();
    let mut server = ctx.socket(zmq::REP)?;
    server.bind("tcp://127.0.0.1:5555")?;

    let zmq_fd = server.get_fd()?;
    println!("[ZMQ] 平台FD: {}", zmq_fd);

    let should_exit = Arc::new(Mutex::new(false));
    let resources = TestResources {
        ctx,
        server,
        should_exit: should_exit.clone(),
    };

    // 客户端线程
    let client_thread = thread::spawn(move || {
        thread::sleep(Duration::from_millis(500));
        let ctx = Context::new();
        let mut client = ctx.socket(zmq::REQ).unwrap();
        client.connect("tcp://127.0.0.1:5555").unwrap();
        client.send(TEST_MESSAGE, 0).unwrap();
        let reply = client.recv_msg(0).unwrap();
        println!("[CLIENT] 收到回复: {:?}", reply);
        *should_exit.lock().unwrap() = true;
    });

    // mio事件循环
    #[cfg(windows)] {
        use std::os::windows::io::FromRawSocket;
        use std::net::TcpStream;
        let std_socket = unsafe { TcpStream::from_raw_socket(zmq_fd as _) };
        let mut mio_socket = mio::net::TcpStream::from_std(std_socket);
        run_event_loop(&mut mio_socket, resources)?;
    }

    #[cfg(unix)] {
        use std::os::unix::io::FromRawFd;
        let mut mio_socket = unsafe { mio::net::TcpStream::from_raw_fd(zmq_fd as _) };
        run_event_loop(&mut mio_socket, resources)?;
    }

    client_thread.join().unwrap();
    Ok(())
}

fn run_event_loop(mio_socket: &mut mio::net::TcpStream, mut resources: TestResources) -> io::Result<()> {
    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(128);
    
    poll.registry().register(
        mio_socket,
        Token(0),
        Interest::READABLE | Interest::WRITABLE
    )?;

    println!("[MIO] 注册成功");
    
    loop {
        poll.poll(&mut events, Some(Duration::from_millis(100)))?;
        for event in events.iter() {
            if event.is_readable() {
                let msg = resources.server.recv_msg(0)?;
                println!("[SERVER] 收到消息: {:?}", msg);
                assert_eq!(msg.as_str().unwrap(), TEST_MESSAGE);
                resources.server.send("PONG", 0)?;
            }
        }
        
        if *resources.should_exit.lock().unwrap() {
            println!("[SERVER] 收到退出信号");
            break;
        }
    }
    
    // 显式释放资源
    drop(resources.server);
    drop(resources.ctx);
    Ok(())
}

#[test]
fn test_mio_zmq_integration() {
    match setup_mio_with_zmq() {
        Ok(_) => println!("[SUCCESS] 测试通过"),
        Err(e) => panic!("测试失败: {:?}", e)
    }
}