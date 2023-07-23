#![allow(dead_code, unused_variables)]
use std::{
    io::BufRead,
    sync::{atomic::AtomicBool, mpsc::channel, Arc},
    thread::{self, JoinHandle},
    time::Duration,
};

use crate::{
    counter::{Counter, ExternalInternal},
    message::{self, Handler, Init, Message},
};

#[derive(Debug)]
pub struct PeriodicThread {
    handler: Option<JoinHandle<()>>,
    terminate: Arc<AtomicBool>,
}

impl PeriodicThread {
    pub fn new<F>(task: F, period: Duration) -> Self
    where
        F: Fn() -> Result<(), anyhow::Error> + Send + 'static,
    {
        let terminate = Arc::new(AtomicBool::new(false));
        let terminate_clone = terminate.clone();
        let handler = thread::spawn(move || loop {
            if terminate_clone.load(std::sync::atomic::Ordering::Acquire) {
                break;
            }
            if let Err(_) = task() {
                println!("Terminating Periodic thread with error");
                break;
            }
            thread::park_timeout(period);
        });

        PeriodicThread {
            handler: Some(handler),
            terminate,
        }
    }
}

impl Drop for PeriodicThread {
    fn drop(&mut self) {
        self.terminate
            .store(true, std::sync::atomic::Ordering::Release);
        if let Some(handler) = self.handler.take() {
            handler.thread().unpark();
            if let Err(_) = handler.join() {
                println!("Periodic thread join ended with error");
            }
        }
    }
}
