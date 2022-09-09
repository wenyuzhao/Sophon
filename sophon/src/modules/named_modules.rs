use core::ops::Deref;

use atomic::{Atomic, Ordering};

use crate::arch::{Arch, TargetArch};

pub struct NamedModule<T: 'static + ?Sized> {
    instance: Atomic<Option<&'static T>>,
    init: Option<fn()>,
}

impl<T: 'static + ?Sized> NamedModule<T> {
    const UNINIT: Self = Self {
        instance: Atomic::new(None),
        init: None,
    };

    const fn uninit_with_post_initializer(init: fn()) -> Self {
        Self {
            instance: Atomic::new(None),
            init: Some(init),
        }
    }

    pub fn set_instance(&self, instance: &'static T) {
        assert!(self.instance.load(Ordering::SeqCst).is_none());
        self.instance.store(Some(instance), Ordering::SeqCst);
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
        debug_assert!(self.instance.load(Ordering::Relaxed).is_some());
        self.instance.load(Ordering::Relaxed).unwrap()
    }
}

pub static INTERRUPT: NamedModule<dyn interrupt::InterruptController> =
    NamedModule::uninit_with_post_initializer(|| TargetArch::setup_interrupt_table());
pub static SCHEDULER: NamedModule<dyn sched::Scheduler> = NamedModule::UNINIT;
pub static PROCESS_MANAGER: NamedModule<dyn proc::ProcessManager> = NamedModule::UNINIT;
pub static TIMER: NamedModule<dyn interrupt::TimerController> = NamedModule::UNINIT;
pub static VFS: NamedModule<dyn vfs::VFSManager> = NamedModule::UNINIT;
