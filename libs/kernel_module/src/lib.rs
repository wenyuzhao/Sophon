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

pub use ::log::log;
pub use call::ModuleCallHandler;
pub use heap::KernelModuleAllocator;
pub use kernel_module_macros::kernel_module;
pub use service::{KernelService, KernelServiceWrapper};
use syscall::ModuleRequest;

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
    instance_mut.init()
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

#[macro_export]
macro_rules! declare_kernel_module {
    ($name:ident) => {
        #[global_allocator]
        static ALLOCATOR: $crate::KernelModuleAllocator = $crate::KernelModuleAllocator;

        #[no_mangle]
        pub extern "C" fn _start(service: $crate::KernelServiceWrapper) -> isize {
            if $crate::init_kernel_module(service, &$name).is_err() {
                return -1;
            }
            0
        }

        #[panic_handler]
        fn panic(info: &::core::panic::PanicInfo) -> ! {
            log!("{}", info);
            loop {}
        }
    };
}
