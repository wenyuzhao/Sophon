#![feature(asm)]
#![feature(format_args_nl)]
#![feature(global_asm)]
#![feature(panic_info_message)]
#![feature(core_intrinsics)]
#![feature(stmt_expr_attributes)]
#![feature(naked_functions)]
#![no_std]
#![no_main]

extern crate lazy_static;
extern crate spin;
#[allow(unused)]
mod gpio;
#[macro_use]
mod debug;
mod mailbox;
mod fb;
mod random;
mod exception;


global_asm!(include_str!("./boot.S"));


pub fn wait_forever() -> ! {
    loop {
        unsafe { asm!("wfe" :::: "volatile") }
    }
}

#[no_mangle]
pub extern "C" fn kmain() -> ! {
    debug!("Hello, Raspberry PI!");
    {
        let mut fb = fb::FRAME_BUFFER.lock();
        fb.init();
        fb.clear(fb::Color::rgba(0x37474FFF));
    }
    debug!("Random: {} {} {}", random::random(0, 100), random::random(0, 100), random::random(0, 100));
    debug!("Current execution level: {}", unsafe {
        let el: u64;
        asm!("mrs x0, CurrentEL" : "={x0}" (el) :: "x0");
        (el >> 2) & 3
    });
    // Manually trigger a pauge fault
    // unsafe { *(0xdeadbeef as *mut u8) = 0; }
    wait_forever();
}



#[cfg(not(feature="rls"))]
#[panic_handler]
fn panic(info: &::core::panic::PanicInfo) -> ! {
    debug!("{}", info);
    loop {}
}