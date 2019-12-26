mod ipc;
mod system;
use crate::task::Message;
use crate::arch::*;

const HANDLERS: [fn (m: &Message); 2] = [
    system::task::fork,
    system::task::exit,
];


pub extern fn main() -> ! {
    println!("Kernel process start");
    loop {
        debug_assert!(Target::Interrupt::is_enabled());
        let m = ipc::receive(None);
        println!("Kernel received {:?}", m);
        HANDLERS[m.kind](&m);
    }
}

pub extern fn idle() -> ! {
    loop {
        unsafe { asm!("wfe"); }
    }
}