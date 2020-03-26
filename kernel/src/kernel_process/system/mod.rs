// pub mod task;
pub mod mem;

use core::marker::PhantomData;
use super::KernelTask;
use crate::AbstractKernel;
use crate::arch::*;
use proton::task::*;
use proton::kernel_call::KernelCall;


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
            let m = Message::receive(None);
            debug!(K: "Kernel received {:?}", m);
            let kind: KernelCall = unsafe { ::core::mem::transmute(m.kind) };
            match kind {
                KernelCall::MapPhysicalMemory => mem::map_physical_memory::<K>(&m),
                _ => {}
            }
            //     println!("Kernel received {:?}", m);
            //     HANDLERS[m.kind](&m);
            // }
        }
    }
}