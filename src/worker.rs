use std::sync::{Arc, Mutex};
use std::sync::mpsc::Receiver;
use std::thread;
use std::fmt;

/// Message type for worker communication
pub enum Message {
    /// New job to execute
    NewJob(Box<dyn FnOnce() + Send + 'static>),
    /// Termination signal
    Terminate,
}

/// Worker thread implementation
pub struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    /// Create a new worker with the given ID and job receiver
    pub fn new(id: usize, receiver: Arc<Mutex<Receiver<Message>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let message = match receiver.lock() {
                Ok(guard) => match guard.recv() {
                    Ok(msg) => msg,
                    Err(_) => break, // Channel disconnected
                },
                Err(_) => break, // Mutex poisoned
            };

            match message {
                Message::NewJob(job) => {
                    log::debug!("Worker {} executing job", id);
                    let start = std::time::Instant::now();
                    job();
                    let duration = start.elapsed();
                    log::debug!("Worker {} finished job in {:?}", id, duration);
                }
                Message::Terminate => {
                    log::debug!("Worker {} received terminate signal", id);
                    break;
                }
            }
        });

        Worker {
            id,
            thread: Some(thread),
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