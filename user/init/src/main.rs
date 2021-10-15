#![feature(asm)]
#![feature(format_args_nl)]
#![feature(default_alloc_error_handler)]
#![no_std]
#![no_main]

#[macro_use]
extern crate log;

use heap::NoAlloc;
use ipc::log::UserLogger;
use ipc::scheme::{Mode, Resource};

#[global_allocator]
static ALLOCATOR: NoAlloc = NoAlloc;

#[no_mangle]
pub extern "C" fn _start(_argc: isize, _argv: *const *const u8) -> isize {
    UserLogger::init();
    log!("Init process start (user mode)");
    let resource = Resource::open("scheme-test:/test", 0, Mode::ReadWrite).unwrap();
    let mut data = [0u8; 100];
    loop {
        let len = resource.read(&mut data).unwrap();
        log!(
            "[init] read from scheme-test -> {:?}",
            core::str::from_utf8(&data[..len])
        );
        resource.write("hello, world").unwrap();
    }
    // Resource::open("proc:/me/exit", 0, Mode::ReadWrite)
    //     .unwrap()
    //     .write(&[])
    //     .unwrap();
    // unreachable!();
}

#[panic_handler]
fn panic(info: &::core::panic::PanicInfo) -> ! {
    log!("{}", info);
    loop {}
}
