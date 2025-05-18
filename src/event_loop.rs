use mio::{Events, Interest, Poll, Registry, Token};
use std::io;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

/// Event loop for handling I/O events
use std::fmt;

pub struct EventLoop {
    registry: Registry,
    events: Events,
    sockets: HashMap<Token, Arc<Mutex<dyn mio::event::Source + Send + Sync>>>,
    next_token: usize,
}

impl EventLoop {
    /// Create a new event loop
    pub fn new() -> io::Result<Self> {
        let poll = mio::Poll::new()?;
        let registry = poll.registry().try_clone()?;
        let events = Events::with_capacity(1024);
        
        Ok(EventLoop {
            registry,
            events,
            sockets: HashMap::new(),
            next_token: 0,
        })
    }
    

    /// Register a socket with the event loop
    pub fn register(
        &mut self,
        socket: Arc<Mutex<dyn mio::event::Source + Send + Sync>>,
        interests: Interest,
    ) -> io::Result<Token> {
        let token = Token(self.next_token);
        self.next_token += 1;
        
        {
            let mut guard = socket.lock().unwrap();
            self.registry.register(
                &mut *guard,
                token,
                interests,
            )?;
        }
        
        self.sockets.insert(token, socket);
        Ok(token)
    }

    /// Deregister a socket from the event loop
    pub fn deregister(&mut self, token: Token) -> io::Result<()> {
        if let Some(socket) = self.sockets.remove(&token) {
            let mut guard = socket.lock().unwrap();
            self.registry.deregister(&mut *guard)?;
        }
        Ok(())
    }

    /// Poll for events with the given timeout
    pub fn poll(&mut self, timeout: Option<std::time::Duration>) -> io::Result<&Events> {
        let mut poll = match Poll::new() {
            Ok(poll) => poll,
            Err(e) => return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("failed to create Poll instance: {}", e)
            )),
        };
        poll.poll(&mut self.events, timeout)?;
        Ok(&self.events)
    }

    /// Get a socket by token
    pub fn get_socket(&self, token: Token) -> Option<&Arc<Mutex<dyn mio::event::Source + Send + Sync>>> {
        self.sockets.get(&token)
    }
}

impl fmt::Debug for EventLoop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventLoop")
            .field("registry", &self.registry)
            .field("events", &self.events)
            .field("next_token", &self.next_token)
            // 跳过sockets字段的调试输出
            .finish()
    }
}