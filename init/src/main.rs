#![feature(asm)]
#![feature(format_args_nl)]
#![feature(default_alloc_error_handler)]
#![no_std]
#![no_main]

#[macro_use]
extern crate sophon;

use sophon::{
    task::{Message, TaskId},
    user::ipc,
    utils::no_alloc::NoAlloc,
};

#[global_allocator]
static ALLOCATOR: NoAlloc = NoAlloc;

#[no_mangle]
pub extern "C" fn _start(_argc: isize, _argv: *const *const u8) -> isize {
    log!("Init process start (user mode)");
    let mut i = 0usize;
    loop {
        log!("Init Ping {}", i);
        ipc::send(Message::new(TaskId::NULL, TaskId::KERNEL).with_data(i));
        let response = ipc::receive(None);
        log!("Init Pong {}", response.get_data::<usize>());
        i = response.get_data::<usize>() + 1;
    }
}

#[panic_handler]
fn panic(info: &::core::panic::PanicInfo) -> ! {
    log!("{}", info);
    loop {}
}
