use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::AtomicBool,
};

use alloc::{sync::Arc, vec::Vec};
use atomic::Ordering;
use mutex::AbstractMonitor;
use proc::TaskId;

use super::{AbstractScheduler, SCHEDULER};

pub struct SysLock {
    is_locked: AtomicBool,
    waiters: spin::Mutex<Vec<TaskId>>,
}

impl SysLock {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            is_locked: AtomicBool::new(false),
            waiters: Default::default(),
        })
    }

    pub fn is_locked(&self) -> bool {
        self.is_locked.load(Ordering::SeqCst)
    }

    pub fn lock(&self) {
        let _guard = interrupt::uninterruptible();
        let task = SCHEDULER.get_current_task_id().unwrap();
        while self.is_locked.fetch_or(true, Ordering::SeqCst) {
            self.waiters.lock().push(task);
            syscall::wait();
        }
    }

    pub fn unlock(&self) {
        let _guard = interrupt::uninterruptible();
        self.is_locked.store(false, Ordering::SeqCst);
        let mut waiters = self.waiters.lock();
        for t in &*waiters {
            if let Some(task) = SCHEDULER.get_task_by_id(*t) {
                SCHEDULER.wake_up(task)
            }
        }
        waiters.clear()
    }
}

pub struct SysMonitor {
    lock: Arc<SysLock>,
    waiters: spin::Mutex<Vec<TaskId>>,
}

impl SysMonitor {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            lock: SysLock::new(),
            waiters: Default::default(),
        })
    }

    pub fn is_locked(&self) -> bool {
        self.lock.is_locked()
    }
}

impl AbstractMonitor for SysMonitor {
    fn lock(&self) {
        self.lock.lock()
    }

    fn unlock(&self) {
        self.lock.unlock()
    }

    fn wait(&self) {
        let _guard = interrupt::uninterruptible();
        {
            let mut waiters = self.waiters.lock();
            let task = SCHEDULER.get_current_task_id().unwrap();
            self.lock.unlock();
            waiters.push(task);
        }
        syscall::wait();
        self.lock.lock();
    }

    fn notify(&self) {
        let _guard = interrupt::uninterruptible();
        let mut waiters = self.waiters.lock();
        for t in &*waiters {
            if let Some(task) = SCHEDULER.get_task_by_id(*t) {
                SCHEDULER.wake_up(task)
            }
        }
        waiters.clear()
    }
}

pub struct SysMutex<T> {
    monitor: Arc<SysMonitor>,
    value: UnsafeCell<T>,
}

impl<T> SysMutex<T> {
    pub fn new(value: T) -> Arc<Self> {
        Arc::new(Self {
            monitor: SysMonitor::new(),
            value: UnsafeCell::new(value),
        })
    }

    pub fn lock<'a>(self: &'a Self) -> SysMutexGuard<'a, T> {
        self.monitor.lock();
        SysMutexGuard { mutex: self }
    }
}

pub struct SysMutexGuard<'a, T: 'a> {
    mutex: &'a SysMutex<T>,
}

impl<'a, T: 'a> Deref for SysMutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mutex.value.get() }
    }
}

impl<'a, T: 'a> DerefMut for SysMutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mutex.value.get() }
    }
}

impl<'a, T: 'a> Drop for SysMutexGuard<'a, T> {
    fn drop(&mut self) {
        self.mutex.monitor.unlock();
    }
}

pub struct SysCondvar {
    monitor: Arc<SysMonitor>,
}

impl SysCondvar {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            monitor: SysMonitor::new(),
        })
    }

    pub fn wait<'a, T>(self: &Arc<Self>, guard: SysMutexGuard<'a, T>) {
        self.monitor.lock();
        drop(guard);
        self.monitor.wait();
        self.monitor.unlock();
    }

    pub fn notify(self) {
        self.monitor.lock();
        self.monitor.notify();
        self.monitor.unlock();
    }
}
