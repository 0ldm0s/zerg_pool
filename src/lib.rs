//! zerg_pool - High-performance thread pool with event loop integration
//!
//! Provides:
//! - Lock-free task queue
//! - Worker thread pool
//! - Mio-based event loop
//! - Async/await compatible interface

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender};

mod worker;
mod event_loop;

/// Main thread pool structure
#[derive(Debug)]
pub struct ZergPool {
    workers: Vec<worker::Worker>,
    sender: Sender<worker::Message>,
    event_loop: Arc<Mutex<event_loop::EventLoop>>,
}

impl ZergPool {
    /// Create a new thread pool with the given size
    ///
    /// # Examples
    ///
    /// ```
    /// use zerg_pool::ZergPool;
    ///
    /// let pool = ZergPool::new(4).unwrap();
    /// ```
    pub fn new(size: usize) -> Result<Self, PoolCreationError> {
        assert!(size > 0);

        let (sender, receiver) = channel();
        let receiver = Arc::new(Mutex::new(receiver));
        
        let mut workers = Vec::with_capacity(size);
        for id in 0..size {
            workers.push(worker::Worker::new(id, Arc::clone(&receiver)));
        }

        let event_loop = Arc::new(Mutex::new(event_loop::EventLoop::new()?));

        Ok(ZergPool {
            workers,
            sender,
            event_loop,
        })
    }

    /// Execute a function on the thread pool
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.sender.send(worker::Message::NewJob(job)).unwrap();
    }

    /// Get a reference to the event loop for socket registration
    pub fn event_loop(&self) -> &Arc<Mutex<event_loop::EventLoop>> {
        &self.event_loop
    }
}

impl Drop for ZergPool {
    fn drop(&mut self) {
        log::debug!("Sending terminate message to all workers");
        
        for _ in &self.workers {
            self.sender.send(worker::Message::Terminate).unwrap();
        }
    }
}

/// Thread pool creation error
#[derive(Debug)]
pub enum PoolCreationError {
    /// Failed to initialize event loop
    EventLoopInitFailed,
    /// IO error occurred
    IoError(std::io::Error),
}

impl From<std::io::Error> for PoolCreationError {
    fn from(err: std::io::Error) -> Self {
        PoolCreationError::IoError(err)
    }
}
