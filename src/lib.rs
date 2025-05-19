//! zerg_pool - High-performance thread pool with event loop integration
//!
//! Provides:
//! - Lock-free task queue
//! - Worker thread pool
//! - Mio-based event loop
//! - Async/await compatible interface
//! - ZeroMQ + Protobuf communication protocol

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod proto;

use std::sync::{Arc, Mutex};
use crossbeam::channel::{unbounded, Sender};
use zmq::Context;
use crate::proto::zergpool::{Response, response::Result as ResponseResult};

pub mod worker;
pub mod zerg;
pub mod event_loop;

pub use self::{worker::Worker};

/// Global ZMQ context
static ZMQ_CONTEXT: once_cell::sync::Lazy<Context> = once_cell::sync::Lazy::new(|| {
    Context::new()
});

/// Main thread pool structure
#[derive(Debug)]
pub struct ZergPool {
    workers: Vec<worker::Worker>,
    task_sender: Sender<worker::Message>,
    zmq_sender: Sender<proto::zergpool::Response>,
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

        let (task_sender, _task_receiver) = unbounded();
        let (zmq_sender, _zmq_receiver) = unbounded();
        
        let mut workers = Vec::with_capacity(size);
        for id in 0..size {
            workers.push(worker::Worker::new(id));
            log::debug!("Created worker {}", id);
        }

        let event_loop = Arc::new(Mutex::new(event_loop::EventLoop::new()?));

        Ok(ZergPool {
            workers,
            task_sender,
            zmq_sender,
            event_loop,
        })
    }

    /// Execute a function on the thread pool
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.task_sender.send(worker::Message::Task(job)).unwrap();
    }

    /// Send ZMQ response back to client
    pub fn send_response(&self, response: proto::zergpool::Response) {
        self.zmq_sender.send(response).unwrap();
    }

    /// Get a reference to the event loop for socket registration
    pub fn event_loop(&self) -> &Arc<Mutex<event_loop::EventLoop>> {
        &self.event_loop
    }

    /// Get the global ZMQ context
    pub fn zmq_context() -> &'static Context {
        &ZMQ_CONTEXT
    }

    /// Handle incoming request and generate response
    fn handle_request(&self, task: proto::zergpool::Task) {
        let mut response = Response::default();
        response.worker_id = "worker_0".to_string();
        response.result = Some(ResponseResult::Output(task.payload.clone()));
        self.send_response(response);
    }
}

impl Drop for ZergPool {
    fn drop(&mut self) {
        log::debug!("Sending terminate message to all workers");
        
        for _ in &self.workers {
            self.task_sender.send(worker::Message::Terminate).unwrap();
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
