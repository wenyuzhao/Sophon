#![feature(asm)]
#![feature(format_args_nl)]
#![no_std]
#![no_main]

#[macro_use]
mod syscall;
#[macro_use]
mod log;


#[no_mangle]
pub extern fn _start(_argc: isize, _argv: *const *const u8) -> isize {
    log!("Hello from init process!");
    loop {}
}

#[panic_handler]
fn panic(_info: &::core::panic::PanicInfo) -> ! {
    loop {}
}
