// pub mod task;
// pub mod mem;

use core::marker::PhantomData;
use super::KernelTask;
use crate::AbstractKernel;
use crate::arch::*;
use proton::task::*;


pub struct System<K: AbstractKernel> {
    // handlers: [fn (m: &Message); KernelCall::COUNT],
    phantom: PhantomData<K>,
}

impl <K: AbstractKernel> System<K> {
    pub fn new() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}


impl <K: AbstractKernel> KernelTask for System<K> {
    fn run(&mut self) -> ! {
        debug!(K: "Kernel process start");
        loop {
            debug_assert!(<K::Arch as AbstractArch>::Interrupt::is_enabled());
            debug!(K: "Kernel process...");
            let _m = Message::receive(None);
            //     println!("Kernel received {:?}", m);
            //     HANDLERS[m.kind](&m);
            // }
        }
    }
}