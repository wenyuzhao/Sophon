#![feature(format_args_nl)]
#![feature(default_alloc_error_handler)]
#![no_std]
#![no_main]

#[macro_use]
extern crate log;

use kernel_module::{KernelService, KernelServiceWrapper};

// static SERVICE: &'static dyn KernelService = &KernelService;

#[no_mangle]
pub extern "C" fn _start(service: KernelServiceWrapper) -> isize {
    kernel_module::init(service);
    log!("Hello, KM");
    return 0;
}

#[panic_handler]
fn panic(info: &::core::panic::PanicInfo) -> ! {
    log!("{}", info);
    loop {}
}
