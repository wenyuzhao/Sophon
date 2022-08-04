#![feature(format_args_nl)]
#![feature(default_alloc_error_handler)]
#![no_std]
#![no_main]

#[macro_use]
extern crate log;

use heap::UserHeap;
use syscall::UserLogger;

#[global_allocator]
static ALLOCATOR: UserHeap = UserHeap::new();

#[no_mangle]
pub extern "C" fn _start(_argc: isize, _argv: *const *const u8) -> isize {
    UserLogger::init();
    ALLOCATOR.init();
    let dir = vfs::open("/etc").unwrap();
    for i in 0..100 {
        if let Ok(Some(x)) = vfs::readdir(dir, i) {
            log!("{}", x);
        } else {
            break;
        }
    }
    syscall::exit()
}

#[panic_handler]
fn panic(info: &::core::panic::PanicInfo) -> ! {
    log!("{}", info);
    syscall::exit();
}
