#![feature(asm)]
#![feature(format_args_nl)]
#![feature(default_alloc_error_handler)]
#![no_std]
#![no_main]

#[macro_use]
extern crate sophon;

use sophon::utils::no_alloc::NoAlloc;

#[global_allocator]
static ALLOCATOR: NoAlloc = NoAlloc;

#[no_mangle]
pub extern "C" fn _start(_argc: isize, _argv: *const *const u8) -> isize {
    log!("Init process start (user mode)");
    loop {}
}

#[panic_handler]
fn panic(info: &::core::panic::PanicInfo) -> ! {
    log!("{}", info);
    loop {}
}
