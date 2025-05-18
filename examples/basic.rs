//! Basic example of zerg_pool usage

use zerg_pool::ZergPool;
use std::thread;
use std::time::Duration;

fn main() {
    // Create a thread pool with 4 workers
    let pool = ZergPool::new(4).unwrap();

    // Execute some tasks
    for i in 0..10 {
        pool.execute(move || {
            println!("Task {} started", i);
            thread::sleep(Duration::from_millis(100));
            println!("Task {} completed", i);
        });
    }

    // Wait for tasks to complete
    thread::sleep(Duration::from_secs(1));
}