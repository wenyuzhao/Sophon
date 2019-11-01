#![feature(asm)]
#![feature(format_args_nl)]
#![feature(global_asm)]
#![feature(panic_info_message)]
#![feature(core_intrinsics)]
#![no_std]
#![no_main]

#[macro_use]
extern crate lazy_static;
extern crate spin;
#[macro_use]
mod uart_debug;

global_asm!(include_str!("./boot.S"));

// #[inline(always)]
pub fn wait_forever() -> ! {
    unsafe {
        loop {
            asm!("wfe" :::: "volatile")
        }
    }
}


#[no_mangle]
pub unsafe extern "C" fn kmain() -> ! {
    debug!("Hello Raspberry PI!");
    wait_forever();
}

#[cfg(not(feature="rls"))]
#[panic_handler]
fn panic(info: &::core::panic::PanicInfo) -> ! {
    debug!("{}", info);
    loop {}
}