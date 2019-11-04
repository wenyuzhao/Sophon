#![feature(asm)]
#![feature(format_args_nl)]
#![feature(global_asm)]
#![feature(panic_info_message)]
#![feature(core_intrinsics)]
#![feature(stmt_expr_attributes)]
#![feature(naked_functions)]
#![allow(unused)]
#![no_std]
#![no_main]

extern crate lazy_static;
extern crate spin;
extern crate cortex_a;
mod gpio;
#[macro_use]
mod debug;
mod mailbox;
mod fb;
mod random;
mod exception;
mod start;
use cortex_a::regs::*;



#[inline(never)]
#[no_mangle]
#[naked]
pub extern "C" fn kmain() -> ! {
    debug!("Hello, Raspberry PI!");
    {
        let mut fb = fb::FRAME_BUFFER.lock();
        fb.init();
        fb.clear(fb::Color::rgba(0x37474FFF));
    }
    debug!("Random: {} {} {}", random::random(0, 100), random::random(0, 100), random::random(0, 100));
    debug!("Current execution level: {}", (CurrentEL.get() & 0b1100) >> 2);
    // Manually trigger a pauge fault
    // unsafe { *(0xdeadbeef as *mut u8) = 0; }
    loop {}
}



#[cfg(not(feature="rls"))]
#[panic_handler]
fn panic(info: &::core::panic::PanicInfo) -> ! {
    debug!("{}", info);
    loop {}
}