#![no_std]

extern crate alloc;

#[allow(unused)]
use core::arch::asm;
use core::ops::{Deref, DerefMut};

use alloc::boxed::Box;

#[inline]
#[cfg(target_arch = "x86_64")]
pub fn enable() {
    unimplemented!()
}

#[inline]
#[cfg(target_arch = "aarch64")]
pub fn enable() {
    unsafe { asm!("msr daifclr, #2") };
}

#[inline]
#[cfg(target_arch = "x86_64")]
pub fn disable() {
    unimplemented!()
}

#[inline]
#[cfg(target_arch = "aarch64")]
pub fn disable() {
    unsafe { asm!("msr daifset, #2") };
}

#[inline]
#[cfg(target_arch = "x86_64")]
pub fn is_enabled() -> bool {
    unimplemented!()
}

#[inline]
#[cfg(target_arch = "aarch64")]
pub fn is_enabled() -> bool {
    unsafe {
        let daif: usize;
        asm!("mrs {}, DAIF", out(reg) daif);
        daif & (1 << 7) == 0
    }
}

#[inline]
pub fn uninterruptible() -> impl Drop {
    struct Guard {
        enabled: bool,
    }
    impl Drop for Guard {
        fn drop(&mut self) {
            if self.enabled {
                enable();
            }
        }
    }
    let enabled = is_enabled();
    if enabled {
        disable();
    }
    Guard { enabled }
}

pub trait UninterruptibleMutex<T> {
    fn lock_uninterruptible(&self) -> Uninterruptible<spin::MutexGuard<'_, T>>;
}

impl<T> UninterruptibleMutex<T> for spin::Mutex<T> {
    #[inline]
    fn lock_uninterruptible(&self) -> Uninterruptible<spin::MutexGuard<'_, T>> {
        Uninterruptible::new(|| self.lock())
    }
}

pub trait UninterruptibleRwLock<T> {
    fn read_uninterruptible(&self) -> Uninterruptible<spin::RwLockReadGuard<'_, T>>;
    fn write_uninterruptible(&self) -> Uninterruptible<spin::RwLockWriteGuard<'_, T>>;
    fn upgradable_read_uninterruptible(
        &self,
    ) -> Uninterruptible<spin::RwLockUpgradableGuard<'_, T>>;
}

impl<T> UninterruptibleRwLock<T> for spin::RwLock<T> {
    #[inline]
    fn read_uninterruptible(&self) -> Uninterruptible<spin::RwLockReadGuard<'_, T>> {
        Uninterruptible::new(|| self.read())
    }
    #[inline]
    fn write_uninterruptible(&self) -> Uninterruptible<spin::RwLockWriteGuard<'_, T>> {
        Uninterruptible::new(|| self.write())
    }
    #[inline]
    fn upgradable_read_uninterruptible(
        &self,
    ) -> Uninterruptible<spin::RwLockUpgradableGuard<'_, T>> {
        Uninterruptible::new(|| self.upgradeable_read())
    }
}

pub struct Uninterruptible<T> {
    value: Option<T>,
    interrupt_is_enabled: bool,
}

impl<T> Uninterruptible<T> {
    #[inline]
    pub fn new(f: impl FnOnce() -> T) -> Self {
        let interrupt_is_enabled = self::is_enabled();
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
        Self {
            interrupt_is_enabled,
            value: Some(f()),
        }
    }
}

impl<T: Deref> Deref for Uninterruptible<T> {
    type Target = <T as Deref>::Target;
    #[inline]
    fn deref(&self) -> &Self::Target {
        self.value.as_ref().unwrap().deref()
    }
}

impl<T: Deref + DerefMut> DerefMut for Uninterruptible<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value.as_mut().unwrap().deref_mut()
    }
}

impl<T> Drop for Uninterruptible<T> {
    #[inline]
    fn drop(&mut self) {
        self.value = None;
        if self.interrupt_is_enabled {
            self::enable();
        }
    }
}

pub type IRQHandler = Box<dyn Fn() -> isize>;
pub type InterruptHandler = Box<dyn Fn(usize, usize, usize, usize, usize, usize) -> isize>;

pub trait InterruptController {
    fn init(&self);
    fn get_active_irq(&self) -> usize;
    fn enable_irq(&self, irq: usize);
    fn disable_irq(&self, irq: usize);
    fn notify_end_of_interrupt(&self);
    fn get_irq_handler(&self, irq: usize) -> Option<&IRQHandler>;
    fn set_irq_handler(&self, irq: usize, handler: IRQHandler);
}
