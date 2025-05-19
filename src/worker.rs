use std::io::{self, Read, Write};
use std::thread;
use std::fmt;
use crossbeam::channel::Sender;
use zmq::{Socket, Context};

/// 新类型包装器用于绕过孤儿规则
pub struct ZmqSocketWrapper(pub Socket);

// 手动实现Debug trait
impl fmt::Debug for ZmqSocketWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ZmqSocketWrapper")
         .field(&"Socket")
         .finish()
    }
}

impl Read for ZmqSocketWrapper {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut msg = zmq::Message::new();
        match self.0.recv(&mut msg, zmq::DONTWAIT) {
            Ok(_) => {
                let data = msg.as_ref();
                let len = std::cmp::min(buf.len(), data.len());
                buf[..len].copy_from_slice(&data[..len]);
                Ok(len)
            }
            Err(e) if e == zmq::Error::EAGAIN => Err(io::ErrorKind::WouldBlock.into()),
            Err(e) => Err(io::Error::new(io::ErrorKind::Other, e)),
        }
    }
}

impl Write for ZmqSocketWrapper {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self.0.send(buf, zmq::DONTWAIT) {
            Ok(_) => Ok(buf.len()),
            Err(e) if e == zmq::Error::EAGAIN => Err(io::ErrorKind::WouldBlock.into()),
            Err(e) => Err(io::Error::new(io::ErrorKind::Other, e)),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Combined trait for readable and writable streams
pub trait ReadWriteTrait: Read + Write + Send {}
impl<T: Read + Write + Send> ReadWriteTrait for T {}

/// Message type for worker communication
pub enum Message {
    /// Task to execute
    Task(Box<dyn FnOnce() + Send + 'static>),
    /// Termination signal
    Terminate,
    /// Response to send back to client (TCP)
    TcpResponse(Box<dyn ReadWriteTrait>, Vec<u8>),
    /// Response to send back to client (ZMQ)
    ZmqResponse(Context, Vec<u8>, String), // (context, message, address)
}

/// Worker thread implementation
pub struct Worker {
    pub id: usize,
    thread: Option<thread::JoinHandle<()>>,
    pub tx: Sender<Message>,
}

impl Clone for Worker {
    fn clone(&self) -> Self {
        Worker {
            id: self.id,
            thread: None,
            tx: self.tx.clone(),
        }
    }
}

impl Worker {
    /// Get the worker's ID
    pub fn id(&self) -> usize {
        self.id
    }

    /// Create a new worker with the given ID and job receiver
    pub fn new(id: usize) -> Worker {
        let (tx, rx) = crossbeam::channel::unbounded();
        log::debug!("Worker {} channel created", id);
        let thread = thread::spawn(move || {
            let rx = rx;
            loop {
                let message = match rx.recv() {
                    Ok(msg) => msg,
                    Err(_) => break, // Channel disconnected
                };

                match message {
                    Message::Task(job) => {
                        log::debug!("Worker {} executing task", id);
                        let start = std::time::Instant::now();
                        job();
                        let duration = start.elapsed();
                        log::debug!("Worker {} finished job in {:?}", id, duration);
                    }
                    Message::Terminate => {
                        log::debug!("Worker {} received terminate signal", id);
                        break;
                    }
                    Message::TcpResponse(mut stream, response) => {
                        log::debug!("Worker {} received TCP response with {} bytes", id, response.len());
                        
                        if let Err(e) = stream.write_all(&response) {
                            log::error!("Worker {} failed to write TCP response: {}", id, e);
                            continue;
                        }

                        if let Err(e) = stream.flush() {
                            log::error!("Worker {} failed to flush TCP stream: {}", id, e);
                            continue;
                        }
                    }
                    Message::ZmqResponse(ctx, response, addr) => {
                        log::debug!("Worker {} received ZMQ response with {} bytes for {}", id, response.len(), addr);
                        
                        // 在worker线程中创建socket
                        let socket = ctx.socket(zmq::DEALER).unwrap();
                        socket.connect(&addr).unwrap();
                        
                        if let Err(e) = socket.send(&response, 0) {
                            log::error!("Worker {} failed to send ZMQ response: {}", id, e);
                            continue;
                        }
                    }
                }
            }
        });

        Worker {
            id,
            thread: Some(thread),
            tx,
        }
    }
}

impl fmt::Debug for Worker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Worker")
            .field("id", &self.id)
            .finish()
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
        }
    }
}