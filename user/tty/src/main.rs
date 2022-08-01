#![feature(format_args_nl)]
#![feature(default_alloc_error_handler)]
#![no_std]
#![no_main]

#[macro_use]
extern crate log;
extern crate alloc;

// use core::arch::asm;
// use core::sync::atomic::{AtomicUsize, Ordering};
use heap::NoAlloc;
use syscall::UserLogger;

#[global_allocator]
static ALLOCATOR: NoAlloc = NoAlloc;

struct TTY;

impl TTY {
    pub fn run(&self) -> ! {
        log!("[[Sophon TTY]]");
        let fd = vfs::open("/dev/tty.serial");
        let mut buf = [0u8; 1];
        loop {
            assert!(fd != -1);
            let len = vfs::read(fd as _, &mut buf);
            assert!(len != -1);
            if len == 0 {
                continue;
            }
            if buf[0] == 127 {
                buf[0] = 8;
                let s = core::str::from_utf8(&buf[0..len as usize]).unwrap();
                print!("{} {}", s, s);
            } else {
                let s = core::str::from_utf8(&buf[0..len as usize]).unwrap();
                print!("{}", s);
            }
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
    loop {
        syscall::wait();
    }
}
