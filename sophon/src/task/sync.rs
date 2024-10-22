use core::sync::atomic::{AtomicBool, Ordering};

use atomic::Atomic;
use crossbeam::queue::SegQueue;
use klib::task::TaskId;

use super::sched::SCHEDULER;

pub struct SysMonitor {
    is_locked: AtomicBool,
    #[allow(unused)]
    owner: Atomic<TaskId>,
    blocked_tasks: SegQueue<TaskId>,
    waiting_tasks: SegQueue<TaskId>,
}

impl SysMonitor {
    pub fn new() -> Self {
        Self {
            is_locked: AtomicBool::new(false),
            owner: Atomic::new(TaskId::NULL),
            blocked_tasks: SegQueue::new(),
            waiting_tasks: SegQueue::new(),
        }
    }

    pub fn lock(&self) {
        let _guard = interrupt::uninterruptible();
        let requester = SCHEDULER.get_current_task_id().unwrap();
        while self.is_locked.fetch_or(true, Ordering::SeqCst) {
            self.blocked_tasks.push(requester);
            SCHEDULER.block_current_task();
        }
    }

    pub fn unlock(&self) {
        let _guard = interrupt::uninterruptible();
        self.is_locked.store(false, Ordering::SeqCst);
        while let Some(t) = self.blocked_tasks.pop() {
            SCHEDULER.unblock_task(t);
        }
    }

    pub fn wait(&self) {
        let _guard = interrupt::uninterruptible();
        let requester = SCHEDULER.get_current_task_id().unwrap();
        self.waiting_tasks.push(requester);
        self.unlock();
        SCHEDULER.block_current_task();
        self.lock();
    }

    pub fn notify_all(&self) {
        let _guard = interrupt::uninterruptible();
        while let Some(t) = self.waiting_tasks.pop() {
            SCHEDULER.unblock_task(t);
        }
    }
}
