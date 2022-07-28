#![no_std]
#![feature(const_type_name)]
#![feature(associated_type_defaults)]
#![feature(type_alias_impl_trait)]
#![feature(generic_associated_types)]

mod heap;
mod log;
mod service;

use core::any::Any;

pub use heap::KernelModuleAllocator;
pub use kernel_module_macros::kernel_module;
pub use service::*;

static mut SERVICE_OPT: Option<&'static dyn KernelService> = None;

pub static SERVICE: spin::Lazy<&'static dyn KernelService> =
    spin::Lazy::new(|| unsafe { *SERVICE_OPT.as_ref().unwrap() });

pub fn init(service: KernelServiceWrapper) {
    unsafe {
        SERVICE_OPT = Some(service.get_service());
        log::init();
    }
}

pub struct Nil;

impl From<(usize, [usize; 3])> for Nil {
    fn from(_: (usize, [usize; 3])) -> Self {
        Nil
    }
}

pub trait KernelModule: 'static {
    const NAME: &'static str = core::any::type_name::<Self>();
    const ENABLE_MODULE_CALL: bool = false;

    fn init(&self) -> anyhow::Result<()>;

    type ModuleCall<'a>: From<(usize, [usize; 3])> = Nil;

    fn module_call<'a>(&'static self, _: Self::ModuleCall<'a>) -> isize {
        -1
    }
}

static mut INSTANCE: Option<&'static dyn Any> = None;

fn instance<T: KernelModule>() -> &'static T {
    unsafe { INSTANCE.unwrap().downcast_ref::<T>().unwrap() }
}

pub fn init_module<T: KernelModule>(m: &'static T) -> anyhow::Result<()> {
    unsafe {
        INSTANCE = Some(m);
    }
    if T::ENABLE_MODULE_CALL {
        extern "C" fn handle_kernel_call<T: KernelModule>(kind: usize, args: [usize; 3]) -> isize {
            instance::<T>().module_call(From::from((kind, args)))
        }
        SERVICE.register_module_call(handle_kernel_call::<T>)
    }

    m.init()
}

#[macro_export]
macro_rules! declare_kernel_module {
    ($name:ident) => {
        #[global_allocator]
        static ALLOCATOR: $crate::KernelModuleAllocator = $crate::KernelModuleAllocator;

        #[no_mangle]
        pub extern "C" fn _start(service: $crate::KernelServiceWrapper) -> isize {
            $crate::init(service);
            if $crate::init_module(&$name).is_err() {
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
