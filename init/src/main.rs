#![feature(asm)]
#![feature(format_args_nl)]
#![no_std]
#![no_main]

#[macro_use]
mod syscall;
#[macro_use]
mod log;
use syscall::SysCall;

#[no_mangle]
pub extern fn _start(_argc: isize, _argv: *const *const u8) -> isize {
    log!("Hello from init process!");
    let id = syscall!(SysCall::Fork);
    log!("Hello from init process! <{}>", id);
    for i in 0..100 {
        log!("Hello from init process! <{}>", id);
    }
    // if id == 0 {
    //     log!("Child process exit...");
    //     syscall!(SysCall::Exit, 0);
    // }
    loop {
        log!("Hello from init process! <{}>", id);
        for i in 0..100000 {
            // unsafe { asm!("nop") }
        }
    }
}

#[panic_handler]
fn panic(_info: &::core::panic::PanicInfo) -> ! {
    loop {}
}
