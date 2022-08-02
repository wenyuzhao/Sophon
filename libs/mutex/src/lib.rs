#![no_std]
#![feature(format_args_nl)]

extern crate alloc;

use alloc::sync::Arc;

pub trait AbstractMonitor {
    fn lock(&self);
    fn unlock(&self);
    fn wait(&self);
    fn notify(&self);
}

pub struct Monitor(Arc<dyn AbstractMonitor>);

impl Monitor {
    pub fn new(m: Arc<dyn AbstractMonitor>) -> Self {
        Self(m)
    }
    pub fn lock(&self) {
        self.0.lock();
    }
    pub fn unlock(&self) {
        self.0.unlock();
    }
    pub fn wait(&self) {
        self.0.wait();
    }
    pub fn notify(&self) {
        self.0.notify();
    }
}
