use core::sync::atomic::AtomicBool;

use alloc::sync::Arc;
use atomic::Ordering;
use mutex::AbstractMonitor;
use proc::TaskId;

use super::{AbstractScheduler, SCHEDULER};

pub struct SysMonitor {
    is_locked: AtomicBool,
    task: TaskId,
}

impl SysMonitor {
    pub fn new() -> Arc<Self> {
        let task = SCHEDULER.get_current_task_id().unwrap();
        Arc::new(Self {
            is_locked: AtomicBool::new(false),
            task,
        })
    }
}

impl AbstractMonitor for SysMonitor {
    fn lock(&self) {
        while self.is_locked.fetch_or(true, Ordering::SeqCst) {
            self.wait();
        }
    }
    fn unlock(&self) {
        SCHEDULER.wake_up(SCHEDULER.get_task_by_id(self.task).unwrap());
        self.is_locked.store(false, Ordering::SeqCst);
    }
    fn wait(&self) {
        syscall::wait();
    }
    fn notify(&self) {
        SCHEDULER.wake_up(SCHEDULER.get_task_by_id(self.task).unwrap());
    }
}
