use core::{cell::UnsafeCell, ops::Deref};

use crate::arch::{Arch, TargetArch};

pub struct NamedModule<T: 'static + ?Sized> {
    instance: UnsafeCell<Option<&'static T>>,
    init: Option<fn()>,
}

impl<T: 'static + ?Sized> NamedModule<T> {
    const UNINIT: Self = Self {
        instance: UnsafeCell::new(None),
        init: None,
    };

    const fn uninit_with_post_initializer(init: fn()) -> Self {
        Self {
            instance: UnsafeCell::new(None),
            init: Some(init),
        }
    }

    pub fn set_instance(&self, instance: &'static T) {
        let slot = unsafe { &mut *self.instance.get() };
        assert!(slot.is_none());
        *slot = Some(instance);
        if let Some(init) = self.init {
            init();
        }
    }
}

unsafe impl<T: 'static + ?Sized> Send for NamedModule<T> {}
unsafe impl<T: 'static + ?Sized> Sync for NamedModule<T> {}

impl<T: 'static + ?Sized> Deref for NamedModule<T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        let slot = unsafe { &mut *self.instance.get() };
        debug_assert!(slot.is_some());
        slot.as_ref().unwrap()
    }
}

pub static INTERRUPT: NamedModule<dyn interrupt::InterruptController> =
    NamedModule::uninit_with_post_initializer(|| TargetArch::setup_interrupt_table());
pub static TIMER: NamedModule<dyn interrupt::TimerController> = NamedModule::UNINIT;
pub static VFS: NamedModule<dyn vfs::VFSManager> = NamedModule::UNINIT;
