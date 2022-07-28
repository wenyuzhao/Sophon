use crate::KernelModule;
use alloc::boxed::Box;
use core::intrinsics::type_id;
use core::marker::PhantomData;

pub trait ModuleCallHandler: Send + Sync {
    fn handle(&self, args: [usize; 4]) -> isize;
}

pub trait ModuleCall {
    fn from(args: [usize; 4]) -> Self;
    fn handle(self) -> anyhow::Result<isize>;
}

impl ModuleCall for ! {
    fn from(_args: [usize; 4]) -> Self {
        unreachable!()
    }
    fn handle(self) -> anyhow::Result<isize> {
        unreachable!()
    }
}

pub(crate) fn register_module_call<T: KernelModule>() {
    if type_id::<T::ModuleCall<'static>>() == type_id::<!>() {
        return;
    }
    struct HandlerImpl<T: KernelModule> {
        _marker: PhantomData<T>,
    }
    impl<T: KernelModule> ModuleCallHandler for HandlerImpl<T> {
        fn handle<'a>(&'a self, args: [usize; 4]) -> isize {
            let call = <T::ModuleCall<'a> as ModuleCall>::from(args);
            call.handle().unwrap_or_else(|_| -1)
        }
    }
    let handler: &'static HandlerImpl<T> = Box::leak(Box::new(HandlerImpl {
        _marker: PhantomData,
    }));
    crate::SERVICE.register_module_call_handler(handler);
}
