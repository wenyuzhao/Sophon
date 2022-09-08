#![no_std]
#![feature(const_type_name)]
#![feature(associated_type_defaults)]
#![feature(type_alias_impl_trait)]
#![feature(generic_associated_types)]
#![feature(never_type)]
#![feature(core_intrinsics)]

extern crate alloc;

mod call;
mod heap;
mod log;
mod service;

pub use ::log::*;
pub use call::ModuleCallHandler;
pub use heap::KernelModuleAllocator;
pub use kernel_module_macros::{kernel_module, test};
pub use service::{KernelService, KernelServiceWrapper};
pub use testing;

use alloc::vec::Vec;
use core::ops::Deref;
use syscall::ModuleRequest;
use testing::Tests;

static mut SERVICE_OPT: Option<&'static dyn KernelService> = None;

pub static SERVICE: spin::Lazy<&'static dyn KernelService> =
    spin::Lazy::new(|| unsafe { *SERVICE_OPT.as_ref().unwrap() });

pub fn init_kernel_service(service: KernelServiceWrapper) {
    unsafe {
        SERVICE_OPT = Some(service.get_service());
        log::init();
    }
}

#[inline]
pub fn module_call<'a>(module: &str, request: &'a impl ModuleRequest<'a>) -> isize {
    SERVICE.module_call(module, request.as_raw())
}

pub fn init_kernel_module<T: KernelModule>(
    service: KernelServiceWrapper,
    instance: &'static T,
) -> anyhow::Result<()> {
    init_kernel_service(service);
    call::register_module_call::<T>(instance);
    let instance_mut = unsafe { &mut *(instance as *const T as *mut T) };
    // Initialize the module
    let result = instance_mut.init()?;
    // Register any tests
    if cfg!(sophon_test) {
        let mut guard = testing::TESTS.write();
        let mut tests = Tests::new(guard.name);
        core::mem::swap(&mut tests, &mut guard);
        SERVICE.register_tests(tests);
    }
    Ok(result)
}

pub trait KernelModule: 'static + Send + Sync {
    const NAME: &'static str = core::any::type_name::<Self>();

    type ModuleRequest<'a>: ModuleRequest<'a> = !;

    fn init(&'static mut self) -> anyhow::Result<()>;

    fn handle_module_call<'a>(
        &self,
        _privileged: bool,
        _request: Self::ModuleRequest<'a>,
    ) -> isize {
        -1
    }
}

pub fn handle_panic() -> ! {
    SERVICE.handle_panic()
}

pub struct ProcessorLocalStorage<T: Default> {
    data: Vec<T>,
}

impl<T: Default> ProcessorLocalStorage<T> {
    pub fn new() -> Self {
        let len = SERVICE.num_cores();
        Self {
            data: (0..len).map(|_| T::default()).collect(),
        }
    }

    #[inline(always)]
    pub fn get(&self, index: usize) -> &T {
        &self.data[index]
    }
}

impl<T: Default> Deref for ProcessorLocalStorage<T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.data[SERVICE.current_core()]
    }
}
