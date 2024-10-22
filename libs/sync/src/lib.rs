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
    _raw: OpaqueMutexPointer,
}

impl RawMutex {
    pub fn new() -> Self {
        unimplemented!()
    }

    pub fn lock(&self) {
        unimplemented!()
    }

    pub fn unlock(&self) {
        unimplemented!()
    }
}

impl Drop for RawMutex {
    fn drop(&mut self) {
        unimplemented!()
    }
}

pub struct RawCondvar {
    _raw: OpaqueCondvarPointer,
}

impl RawCondvar {
    pub fn new() -> Self {
        unimplemented!()
    }

    pub fn wait(&self, _mutex: &RawMutex) {
        unimplemented!()
    }

    pub fn notify_all(&self) {
        unimplemented!()
    }
}

impl Drop for RawCondvar {
    fn drop(&mut self) {
        unimplemented!()
    }
}

pub struct SysMonitor {
    _handle: Option<usize>,
}

impl SysMonitor {
    pub fn new() -> Self {
        unreachable!();
    }

    pub fn lock(&self) {
        unreachable!()
    }

    pub fn unlock(&self) {
        unreachable!()
    }

    pub fn wait(&self) {
        unreachable!()
    }

    pub fn notify_all(&self) {
        unreachable!()
    }
}

pub struct Monitor<T> {
    raw: SysMonitor,
    data: UnsafeCell<T>,
}

impl<T> Monitor<T> {
    pub fn new(value: T) -> Self {
        Self {
            raw: SysMonitor::new(),
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
