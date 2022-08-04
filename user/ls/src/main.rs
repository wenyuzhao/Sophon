#![feature(format_args_nl)]
#![feature(default_alloc_error_handler)]
#![no_std]
#![no_main]

#[macro_use]
extern crate log;

extern crate alloc;

use core::ffi::CStr;

use alloc::format;
use heap::UserHeap;
use syscall::UserLogger;

#[global_allocator]
static ALLOCATOR: UserHeap = UserHeap::new();

#[no_mangle]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> isize {
    UserLogger::init();
    ALLOCATOR.init();
    let path = if argc == 0 {
        "."
    } else {
        let c_str: &CStr = unsafe { CStr::from_ptr(argv.read() as _) };
        c_str.to_str().unwrap().trim()
    };
    let dir = vfs::open(path).expect("ERROR: No such file or directory");
    for i in 0..100 {
        if let Ok(Some(x)) = vfs::readdir(dir, i) {
            let child_path = if path == "/" {
                format!("/{}", x)
            } else {
                format!("{}/{}", path, x)
            };
            let fd = vfs::open(&child_path).unwrap();
            if vfs::readdir(fd, 0).is_ok() {
                log!("{}/", x);
            } else {
                log!("{}", x);
            }
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
