use core::ops::{Deref, DerefMut};

use spin::{Mutex, MutexGuard};

// uninterruptible Mutex
pub struct UnintMutex<T> {
    mutex: Mutex<T>,
}

impl<T> UnintMutex<T> {
    pub const fn new(v: T) -> Self {
        Self {
            mutex: Mutex::new(v),
        }
    }

    pub fn lock(&self) -> UnintMutexGuard<'_, T> {
        let interrupt_is_enabled = interrupt::is_enabled();
        UnintMutexGuard {
            mutex_guard: Some(self.mutex.lock()),
            interrupt_is_enabled,
        }
    }
}

pub struct UnintMutexGuard<'a, T: 'a + ?Sized> {
    mutex_guard: Option<MutexGuard<'a, T>>,
    interrupt_is_enabled: bool,
}

impl<'a, T: 'a + ?Sized> Deref for UnintMutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.mutex_guard.as_ref().unwrap()
    }
}

impl<'a, T: 'a + ?Sized> DerefMut for UnintMutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.mutex_guard.as_mut().unwrap()
    }
}

impl<'a, T: 'a + ?Sized> Drop for UnintMutexGuard<'a, T> {
    fn drop(&mut self) {
        self.mutex_guard = None;
        if self.interrupt_is_enabled {
            interrupt::enable();
        }
    }
}
