mod ipc;
mod system;
use crate::task::Message;

const HANDLERS: [fn (m: &Message); 2] = [
    system::task::fork,
    system::task::exit,
];


pub extern fn main() -> ! {
    println!("Kernel process start");
    loop {
        debug_assert!(crate::interrupt::is_enabled());
        let mut m = ipc::receive(None);
        println!("Kernel received {:?}", m);
        HANDLERS[m.kind](&m);
    }
}

pub extern fn idle() -> ! {
    loop {
        unsafe { asm!("wfe"); }
    }
}