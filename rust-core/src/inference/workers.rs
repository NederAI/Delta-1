//! Lightweight worker pool for CPU bound inference tasks.
//!
//! TODO: Add graceful shutdown so dropped pools stop accepting new work.
//! TODO: Surface metrics on queue depth and worker utilisation.

use std::sync::{mpsc, Arc, Mutex};
use std::thread;

type Job = Box<dyn FnOnce() + Send + 'static>;

pub struct Pool {
    tx: mpsc::Sender<Job>,
}

impl Pool {
    pub fn new(size: usize) -> Self {
        let (tx, rx) = mpsc::channel::<Job>();
        let shared_rx = Arc::new(Mutex::new(rx));

        for _ in 0..size {
            let rx = shared_rx.clone();
            thread::spawn(move || loop {
                let job = {
                    let guard = rx.lock().expect("worker mutex poisoned");
                    guard.recv()
                };

                match job {
                    Ok(job) => job(),
                    Err(_) => break,
                }
            });
        }

        // TODO: Track worker handles to allow explicit joins on shutdown.
        Self { tx }
    }

    pub fn submit<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let _ = self.tx.send(Box::new(job));
        // TODO: Propagate backpressure when the queue is saturated.
    }
}

// TODO: Implement Drop to close the channel and await worker completion.
