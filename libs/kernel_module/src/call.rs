use crate::KernelModule;
use alloc::boxed::Box;
use core::intrinsics::type_id;
use syscall::{ModuleRequest, RawModuleRequest};

pub trait ModuleCallHandler: Send + Sync {
    fn handle<'a>(&self, privileged: bool, request: RawModuleRequest<'a>) -> isize;
}

pub(crate) fn register_module_call<T: KernelModule>(module: &'static T) {
    if type_id::<T::ModuleRequest<'static>>() == type_id::<!>() {
        return;
    }
    struct HandlerImpl<T: KernelModule> {
        module: &'static T,
    }
    impl<T: KernelModule> ModuleCallHandler for HandlerImpl<T> {
        fn handle<'a>(&'a self, privileged: bool, raw: RawModuleRequest<'a>) -> isize {
            let request = <T::ModuleRequest<'a> as ModuleRequest>::from_raw(raw);
            self.module.handle_module_call(privileged, request)
        }
    }
    let handler: &'static HandlerImpl<T> = Box::leak(Box::new(HandlerImpl { module }));
    crate::SERVICE.register_module_call_handler(handler);
}
