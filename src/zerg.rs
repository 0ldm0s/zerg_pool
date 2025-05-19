use std::io::{Read, Write};
use crate::worker::ZmqSocketWrapper;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use log::{debug, error, info};
use prost::Message;
use zmq::{Context, Socket, DEALER, ROUTER, POLLIN};
use crate::worker::{Message as WorkerMessage, Worker};
use crate::ZergPool;

impl ZergPool {
    /// Start the ZMQ server with reconnection handling
    pub fn start_zmq_server(&self, addr: String) -> std::io::Result<()> {
        let ctx = Self::zmq_context();
        let workers = self.workers.clone();
        let pool_size = self.workers.len();

        thread::spawn(move || {
            let mut reconnect_attempts = 0;
            const MAX_RECONNECT_ATTEMPTS: u32 = 5;
            const RECONNECT_DELAY: Duration = Duration::from_secs(1);

            loop {
                let router = match ctx.socket(ROUTER) {
                    Ok(s) => s,
                    Err(e) => {
                        error!("Failed to create ROUTER socket: {}", e);
                        thread::sleep(RECONNECT_DELAY);
                        continue;
                    }
                };

                if let Err(e) = router.bind(&addr) {
                    error!("Failed to bind ROUTER socket: {}", e);
                    reconnect_attempts += 1;
                    if reconnect_attempts >= MAX_RECONNECT_ATTEMPTS {
                        error!("Max reconnection attempts reached");
                        break;
                    }
                    thread::sleep(RECONNECT_DELAY);
                    continue;
                }

                let dealer = match ctx.socket(DEALER) {
                    Ok(s) => s,
                    Err(e) => {
                        error!("Failed to create DEALER socket: {}", e);
                        continue;
                    }
                };

                reconnect_attempts = 0;
                info!("ZMQ server started on {}", &addr);

                let mut items = [router.as_poll_item(POLLIN)];
                let mut worker_idx = 0;

                loop {
                    if zmq::poll(&mut items, 1000).is_err() {
                        error!("Poll error, reconnecting...");
                        break;
                    }

                    if items[0].is_readable() {
                        let mut msg = zmq::Message::new();
                        if let Err(e) = router.recv(&mut msg, 0) {
                            error!("Failed to receive ZMQ message: {}", e);
                            continue;
                        }

                        let client_id = msg.as_str().unwrap().to_string();
                        router.recv(&mut msg, 0).unwrap(); // Empty delimiter
                        router.recv(&mut msg, 0).unwrap(); // Actual message

                        let worker = &workers[worker_idx % pool_size];
                        if let Err(e) = worker.tx.send(WorkerMessage::ZmqResponse(
                            ctx.clone(),
                            msg.to_vec(),
                            addr.clone()
                        )) {
                            error!("Failed to send task to worker {}: {}", worker.id(), e);
                        }

                        worker_idx += 1;
                    }
                }
            }
        });

        Ok(())
    }

    /// Handle incoming ZMQ message with error recovery
    fn handle_zmq_message(&self, socket: &Socket, msg: &[u8]) {
        let task = match super::proto::zergpool::Task::decode(msg) {
            Ok(t) => t,
            Err(e) => {
                error!("Failed to parse protobuf: {}", e);
                return;
            }
        };

        debug!("Received task: {:?}", task);
        let response = self.handle_request(task);

        let mut buf = Vec::new();
        if let Err(e) = response.encode(&mut buf) {
            error!("Failed to serialize response: {}", e);
            return;
        }

        // Retry logic for message sending
        for attempt in 0..3 {
            match socket.send(&buf, zmq::DONTWAIT) {
                Ok(_) => break,
                Err(e) if attempt == 2 => {
                    error!("Failed to send ZMQ response after 3 attempts: {}", e);
                }
                Err(e) => {
                    error!("Failed to send ZMQ response (attempt {}): {}", attempt + 1, e);
                    thread::sleep(Duration::from_millis(100 * (attempt + 1) as u64));
                }
            }
        }
    }
}
