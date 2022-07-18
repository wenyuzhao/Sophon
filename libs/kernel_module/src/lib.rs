#![no_std]
#![feature(const_type_name)]
#![feature(associated_type_defaults)]

use core::alloc::GlobalAlloc;
use core::{alloc::Layout, fmt, ops::Deref};
use log::Logger;
use memory::address::Address;

pub use kernel_module_macros::kernel_module;

pub trait KernelService: Send + Sync + 'static {
    fn log(&self, s: &str);
    fn alloc(&self, layout: Layout) -> Option<Address>;
    fn dealloc(&self, address: Address, layout: Layout);
    fn register_module_call(&self, handler: extern "C" fn(kind: usize, args: [usize; 3]) -> isize);
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct KernelServiceWrapper([usize; 2]);

impl KernelServiceWrapper {
    pub fn get_service(self) -> &'static dyn KernelService {
        unsafe { core::mem::transmute(self) }
    }
    pub fn from_service(service: &'static dyn KernelService) -> Self {
        unsafe { core::mem::transmute(service) }
    }
}

impl Deref for KernelServiceWrapper {
    type Target = dyn KernelService;

    fn deref(&self) -> &'static Self::Target {
        self.get_service()
    }
}

impl Logger for &dyn KernelService {
    fn log(&self, message: &str) -> Result<(), fmt::Error> {
        KernelService::log(*self, message);
        Ok(())
    }
}

static mut SERVICE_OPT: Option<&'static dyn KernelService> = None;

pub static SERVICE: spin::Lazy<&'static dyn KernelService> =
    spin::Lazy::new(|| unsafe { *SERVICE_OPT.as_ref().unwrap() });

pub fn init(service: KernelServiceWrapper) {
    unsafe {
        SERVICE_OPT = Some(service.get_service());
        log::init(&*SERVICE);
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

pub struct KernelModuleAllocator;

unsafe impl GlobalAlloc for KernelModuleAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        SERVICE.alloc(layout).unwrap().as_mut_ptr()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        SERVICE.dealloc(ptr.into(), layout)
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
