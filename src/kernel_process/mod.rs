mod system;
use crate::task::Message;
use crate::arch::*;
use proton::KernelCall;

const HANDLERS: [fn (m: &Message); KernelCall::COUNT] = [
    system::task::fork,
    system::task::exit,
    system::mem::physical_memory,
];


pub extern fn main() -> ! {
    println!("Kernel process start");
    loop {
        debug_assert!(Target::Interrupt::is_enabled());
        let m = Message::receive(None);
        println!("Kernel received {:?}", m);
        HANDLERS[m.kind](&m);
    }
}

pub extern fn idle() -> ! {
    loop {
        unsafe { asm!("wfe"); }
    }
}