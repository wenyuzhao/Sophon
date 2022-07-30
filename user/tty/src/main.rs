#![feature(format_args_nl)]
#![feature(default_alloc_error_handler)]
#![no_std]
#![no_main]

#[macro_use]
extern crate log;
extern crate alloc;

use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::vec;
// use core::arch::asm;
// use core::sync::atomic::{AtomicUsize, Ordering};
use heap::NoAlloc;
use syscall::UserLogger;

#[global_allocator]
static ALLOCATOR: NoAlloc = NoAlloc;

struct TTY;

impl TTY {
    pub fn read_command(&self) -> String {
        let mut bytes = vec![0u8; 32];
        loop {
            let mut buf = [0u8; 32];
            let len = syscall::read(0, &mut buf);
            assert!(len != -1);
            if len == 0 {
                break;
            }
            bytes.extend_from_slice(&buf[0..len as usize]);
        }
        core::str::from_utf8(&bytes).unwrap().to_owned()
    }

    pub fn run(&self) -> ! {
        log!("[[Sophon TTY]]");
        loop {
            let command = self.read_command();
            log!("{}", command);
        }
    }
}

#[no_mangle]
pub extern "C" fn _start(_argc: isize, _argv: *const *const u8) -> isize {
    UserLogger::init();
    let tty = TTY;
    tty.run();
}

#[panic_handler]
fn panic(info: &::core::panic::PanicInfo) -> ! {
    log!("{}", info);
    loop {}
}