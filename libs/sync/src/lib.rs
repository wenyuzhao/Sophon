#![no_std]
#![feature(format_args_nl)]

extern crate alloc;

use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
};

use syscall::module_calls::proc::{OpaqueCondvarPointer, OpaqueMutexPointer};

pub trait AbstractRawMutex {
    fn lock(&self);
    fn unlock(&self);
}

pub trait AbstractRawCondvar {
    fn wait(&self, mutex: &dyn AbstractRawMutex);
    fn notify_all(&self);
}

pub struct RawMutex {
    raw: OpaqueMutexPointer,
}

impl RawMutex {
    pub fn new() -> Self {
        Self {
            raw: syscall::mutex_create(),
        }
    }

    pub fn lock(&self) {
        syscall::mutex_lock(self.raw);
    }

    pub fn unlock(&self) {
        syscall::mutex_unlock(self.raw);
    }
}

impl Drop for RawMutex {
    fn drop(&mut self) {
        syscall::mutex_destroy(self.raw);
    }
}

pub struct RawCondvar {
    raw: OpaqueCondvarPointer,
}

impl RawCondvar {
    pub fn new() -> Self {
        Self {
            raw: syscall::condvar_create(),
        }
    }

    pub fn wait(&self, mutex: &RawMutex) {
        syscall::condvar_wait(self.raw, mutex.raw);
    }

    pub fn notify_all(&self) {
        syscall::condvar_notify_all(self.raw);
    }
}

impl Drop for RawCondvar {
    fn drop(&mut self) {
        syscall::condvar_destory(self.raw);
    }
}

pub struct RawMonitor {
    lock: RawMutex,
    cond: RawCondvar,
}

impl RawMonitor {
    pub fn new() -> Self {
        Self {
            lock: RawMutex::new(),
            cond: RawCondvar::new(),
        }
    }

    pub fn lock(&self) {
        self.lock.lock();
    }

    pub fn unlock(&self) {
        self.lock.unlock();
    }

    pub fn wait(&self) {
        self.cond.wait(&self.lock);
    }

    pub fn notify_all(&self) {
        self.cond.notify_all();
    }
}

pub struct Monitor<T> {
    raw: RawMonitor,
    data: UnsafeCell<T>,
}

impl<T> Monitor<T> {
    pub fn new(value: T) -> Self {
        Self {
            raw: RawMonitor::new(),
            data: UnsafeCell::new(value),
        }
    }

    pub fn lock<'a>(self: &'a Self) -> MonitorGuard<'a, T> {
        self.raw.lock();
        MonitorGuard { monitor: self }
    }

    pub fn wait<'a>(self: &'a Self, guard: MonitorGuard<'a, T>) -> MonitorGuard<'a, T> {
        self.raw.wait();
        guard
    }

    pub fn notify_all(&self) {
        self.raw.notify_all()
    }
}

pub struct MonitorGuard<'a, T: 'a> {
    monitor: &'a Monitor<T>,
}

impl<'a, T: 'a> Deref for MonitorGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.monitor.data.get() }
    }
}

impl<'a, T: 'a> DerefMut for MonitorGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.monitor.data.get() }
    }
}

impl<'a, T: 'a> Drop for MonitorGuard<'a, T> {
    fn drop(&mut self) {
        self.monitor.raw.unlock();
    }
}

pub struct Mutex<T> {
    lock: RawMutex,
    data: UnsafeCell<T>,
}

impl<T> Mutex<T> {
    pub fn new(value: T) -> Self {
        Self {
            lock: RawMutex::new(),
            data: UnsafeCell::new(value),
        }
    }

    pub fn lock<'a>(self: &'a Self) -> MutexGuard<'a, T> {
        self.lock.lock();
        MutexGuard { mutex: self }
    }
}

pub struct MutexGuard<'a, T: 'a> {
    mutex: &'a Mutex<T>,
}

impl<'a, T: 'a> Deref for MutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mutex.data.get() }
    }
}

impl<'a, T: 'a> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mutex.data.get() }
    }
}

impl<'a, T: 'a> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        self.mutex.lock.unlock();
    }
}

pub struct Condvar {
    raw: RawCondvar,
}

impl Condvar {
    pub fn new() -> Self {
        Self {
            raw: RawCondvar::new(),
        }
    }

    pub fn wait<'a, T>(self: &Self, guard: MutexGuard<'a, T>) -> MutexGuard<'a, T> {
        self.raw.wait(&guard.mutex.lock);
        guard
    }

    pub fn notify_all(self) {
        self.raw.notify_all()
    }
}
