use core::sync::atomic::AtomicBool;

use alloc::{sync::Arc, vec::Vec};
use mutex::AbstractMonitor;
use proc::TaskId;

use super::{AbstractScheduler, SCHEDULER};

pub struct SysMonitor {
    _is_locked: AtomicBool,
    waiters: spin::Mutex<Vec<TaskId>>,
}

impl SysMonitor {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            _is_locked: AtomicBool::new(false),
            waiters: Default::default(),
        })
    }
}

impl AbstractMonitor for SysMonitor {
    fn lock(&self) {
        // while self.is_locked.fetch_or(true, Ordering::SeqCst) {
        //     self.wait();
        // }
        unimplemented!()
    }
    fn unlock(&self) {
        // SCHEDULER.wake_up(SCHEDULER.get_task_by_id(self.task).unwrap());
        // self.is_locked.store(false, Ordering::SeqCst);
        unimplemented!()
    }
    fn wait(&self) {
        let task = SCHEDULER.get_current_task_id().unwrap();
        self.waiters.lock().push(task);
        syscall::wait();
    }
    fn notify(&self) {
        for t in &*self.waiters.lock() {
            if let Some(task) = SCHEDULER.get_task_by_id(*t) {
                SCHEDULER.wake_up(task)
            }
        }
    }
}

// pub struct Mutex<T> {
//     monitor: Arc<SysMonitor>,
//     value: UnsafeCell<T>,
// }

// impl<T> Mutex<T> {
//     pub fn new(value: T) -> Arc<Self> {
//         Arc::new(Self {
//             monitor: SysMonitor::new(),
//             value: UnsafeCell::new(value),
//         })
//     }
// }
