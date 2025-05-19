#[cfg(windows)]
mod tests {
    use log::debug;
    use std::io::Read;
    use std::net::TcpStream;
    use std::io::Write;
    use std::thread;
    use std::time::Duration;
    use zerg_pool::{ZergConfig, ZergServer};

    #[test]
    fn test_tcp_connection() {
        env_logger::try_init().ok();
        debug!("Starting test_tcp_connection");
        let config1 = ZergConfig::with_tcp_addr("127.0.0.1:12347").unwrap();
        debug!("Created config: {:?}", config1);
        let mut server1 = ZergServer::new(config1).unwrap();
        debug!("Server created successfully");
        
        let (shutdown_sender, shutdown_receiver) = crossbeam::channel::bounded(1);
        let handle = thread::spawn(move || {
            server1.run(Some(shutdown_receiver)).unwrap();
        });

        // Wait for server to start with retries
        let mut connected = false;
        for _ in 0..5 {
            thread::sleep(Duration::from_millis(200));
            if TcpStream::connect("127.0.0.1:12347").is_ok() {
                connected = true;
                break;
            }
        }

        if !connected {
            panic!("Failed to connect to server after multiple attempts");
        }

        // Test basic TCP connection
        let mut stream = TcpStream::connect("127.0.0.1:12347").unwrap();
        stream.write_all(b"test").unwrap();
        debug!("TCP connection successful");
        
        // Send shutdown signal
        shutdown_sender.send(()).unwrap();
        handle.join().unwrap();
    }

    #[test]
    fn test_task_processing() {
        env_logger::try_init().ok();
        debug!("Starting test_task_processing");
        let config1 = ZergConfig::with_tcp_addr("127.0.0.1:12348").unwrap();
        debug!("Created config: {:?}", config1);
        let mut server1 = ZergServer::new(config1).unwrap();
        debug!("Server created successfully");
        
        let (shutdown_sender, shutdown_receiver) = crossbeam::channel::bounded(1);
        let handle = thread::spawn(move || {
            server1.run(Some(shutdown_receiver)).unwrap();
        });

        // Wait for server to start with retries
        let mut connected = false;
        for _ in 0..5 {
            thread::sleep(Duration::from_millis(200));
            if TcpStream::connect("127.0.0.1:12348").is_ok() {
                connected = true;
                break;
            }
        }

        if !connected {
            panic!("Failed to connect to server after multiple attempts");
        }

        // Test task execution
        let mut stream = TcpStream::connect("127.0.0.1:12348").unwrap();
        stream.write_all(b"task").unwrap();
        debug!("Task connection successful");
        
        // Verify task response
        let mut response = [0; 4];
        match stream.read_exact(&mut response) {
            Ok(_) => assert_eq!(&response, b"done", "Task should return 'done' response"),
            Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => {
                // Server may close connection immediately after sending response
                assert_eq!(&response, b"done", "Task should return 'done' response")
            },
            Err(e) => panic!("Unexpected error reading response: {:?}", e),
        }

        // Send shutdown signal
        shutdown_sender.send(()).unwrap();
        handle.join().unwrap();
    }

    #[test]
    fn test_error_handling() {
        debug!("Starting test_error_handling");
        
        // Test invalid address format
        let invalid_addr_config = ZergConfig::with_tcp_addr("invalid_address");
        match invalid_addr_config {
            Ok(_) => panic!("Expected error for invalid address"),
            Err(e) => debug!("Got expected error for invalid address: {:?}", e),
        }

        // Test empty address
        let empty_addr_config = ZergConfig::with_tcp_addr("");
        match empty_addr_config {
            Ok(_) => panic!("Expected error for empty address"),
            Err(e) => debug!("Got expected error for empty address: {:?}", e),
        }

        // Test port already in use
        let config1 = ZergConfig::with_tcp_addr("127.0.0.1:12349").unwrap();
        debug!("Created config: {:?}", config1);
        let mut server1 = ZergServer::new(config1).unwrap();
        debug!("Server created successfully");
        
        let (shutdown_sender, shutdown_receiver) = crossbeam::channel::bounded(1);
        let handle = thread::spawn(move || {
            server1.run(Some(shutdown_receiver)).unwrap();
        });

        // Give server time to start
        thread::sleep(Duration::from_millis(500));

        let config2 = ZergConfig::with_tcp_addr("127.0.0.1:12349").unwrap();
        match ZergServer::new(config2) {
            Ok(_) => panic!("Expected error for port in use"),
            Err(e) => debug!("Got expected error for port in use: {:?}", e),
        }
        
        // Send shutdown signal
        shutdown_sender.send(()).unwrap();
        handle.join().unwrap();
    }
}