#![feature(default_alloc_error_handler)]
#![no_std]
#![no_main]

#[macro_use]
extern crate user;

#[no_mangle]
pub extern "C" fn _start(_argc: isize, _argv: *const *const u8) -> isize {
    println!("Hello, world!");
    user::sys::exit()
}
