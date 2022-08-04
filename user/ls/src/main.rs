#![feature(default_alloc_error_handler)]
#![no_std]
#![no_main]

#[macro_use]
extern crate user;

extern crate alloc;

use core::ffi::CStr;

use alloc::format;

#[no_mangle]
pub extern "C" fn _start(argc: isize, argv: *const *const u8) -> isize {
    let path = if argc == 0 {
        "."
    } else {
        let c_str: &CStr = unsafe { CStr::from_ptr(argv.read() as _) };
        c_str.to_str().unwrap().trim()
    };
    let dir = user::sys::open(path).expect("ERROR: No such file or directory");
    for i in 0..100 {
        if let Ok(Some(x)) = user::sys::readdir(dir, i) {
            let child_path = if path == "/" {
                format!("/{}", x)
            } else {
                format!("{}/{}", path, x)
            };
            let fd = user::sys::open(&child_path).unwrap();
            if user::sys::readdir(fd, 0).is_ok() {
                println!("{}/", x);
            } else {
                println!("{}", x);
            }
        } else {
            break;
        }
    }
    user::sys::exit()
}
