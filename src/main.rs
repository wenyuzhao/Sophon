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


global_asm!(include_str!("./boot.S"));


pub fn wait_forever() -> ! {
    loop {
        unsafe { asm!("wfe" :::: "volatile") }
    }
}

#[no_mangle]
pub extern "C" fn kmain() -> ! {
    unsafe {
        debug!("1234");
        // *(0xdeadbeef as *mut u8) = 0;
    }
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
    wait_forever();
}


#[no_mangle]
#[naked]
pub extern "C" fn exc_handler() -> ! {
    loop {
        unsafe { asm!("wfe" :::: "volatile") }
    }
}


#[cfg(not(feature="rls"))]
#[panic_handler]
fn panic(info: &::core::panic::PanicInfo) -> ! {
    debug!("{}", info);
    loop {}
}