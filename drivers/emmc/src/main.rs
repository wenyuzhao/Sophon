#![feature(asm)]
#![feature(format_args_nl)]
#![no_std]
#![no_main]

extern crate proton;

#[macro_use]
mod log;
use proton::*;

#[no_mangle]
pub extern fn _start(_argc: isize, _argv: *const *const u8) -> isize {
    loop {}
}

#[panic_handler]
#[cfg(not(feature="rls"))]
fn panic(info: &::core::panic::PanicInfo) -> ! {
    log!("{}", info);
    loop {}
}
