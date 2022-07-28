#![no_std]
#![feature(const_type_name)]
#![feature(associated_type_defaults)]

mod heap;
mod log;
mod service;

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

pub trait KernelModule {
    const NAME: &'static str = core::any::type_name::<Self>();
    const ENABLE_MODULE_CALL: bool = false;

    type ModuleCallKind = usize;

    fn init(&self) -> anyhow::Result<()>;

    fn module_call(&'static self, _kind: Self::ModuleCallKind, _args: [usize; 3]) -> isize {
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
            use $crate::KernelModule;
            $crate::init(service);
            fn enable_module_call<T: $crate::KernelModule>(_: &T) -> bool {
                T::ENABLE_MODULE_CALL
            }
            if enable_module_call(&$name) {
                extern "C" fn handle_kernel_call(kind: usize, args: [usize; 3]) -> isize {
                    $name.module_call(unsafe { core::mem::transmute(kind) }, args)
                }
                $crate::SERVICE.register_module_call(handle_kernel_call)
            }
            if $name.init().is_err() {
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
