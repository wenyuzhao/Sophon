#![no_std]
#![no_main]
#![feature(global_asm)]
#![feature(llvm_asm)]
#![feature(alloc)]
#![feature(default_alloc_error_handler)]
#![feature(format_args_nl)]
#![feature(alloc_error_handler)]

use core::panic::PanicInfo;
use uefi::prelude::*;
#[macro_use] mod log;

static mut SYSTEM_TABLE: Option<SystemTable<Boot>> = None;

#[no_mangle]
pub extern "C" fn efi_main(image: Handle, st: SystemTable<Boot>) -> Status {
    unsafe { SYSTEM_TABLE = Some(st); }
    log!("xxx");
    loop {}
}

#[panic_handler]
fn panic(_panic: &PanicInfo<'_>) -> ! {
    loop {}
}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}