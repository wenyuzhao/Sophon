#![feature(asm, llvm_asm)]
#![feature(format_args_nl)]
#![no_std]
#![no_main]

extern crate proton;

#[macro_use]
mod log;

#[no_mangle]
pub extern "C" fn _start(_argc: isize, _argv: *const *const u8) -> isize {
    // unsafe { llvm_asm!("1:  b 1b") }
    log!("Init process start (user mode)");

    // let msg = Message {
    //     sender: 0,
    //     receiver: 0,
    //     kind: 233,
    //     data: [0; 16]
    // };

    // msg.send();

    // loop {
    //     let mut m = Message::receive(None);
    //     if m.kind == 233 {
    //         log!("Init received #233: {:?}", m.data[0]);
    //         m.data[0] += 1;
    //         m.receiver = m.sender;
    //         m.send()
    //     }
    // }

    // let id = KernelCall::fork().unwrap();
    // log!("Fork return -> {:?}", id);
    loop {}
    // unreachable!();
    // let id = syscall!(SysCall::Fork);
    // log!("Hello from init process! <{}>", id);
    // // loop {}
    // for i in 0..100 {
    //     log!("Hello from init process! <{}>", id);
    // }

    // if id == 0 {
    //     log!("Child process exit...");
    //     syscall!(SysCall::Exit, 0usize);
    // }
    // loop {
    //     log!("Hello from init process! <{}>", id);
    //     for i in 0..100000 {
    //         // unsafe { llvm_asm!("nop") }
    //     }
    // }
}

#[panic_handler]
fn panic(info: &::core::panic::PanicInfo) -> ! {
    log!("{}", info);
    loop {}
}
