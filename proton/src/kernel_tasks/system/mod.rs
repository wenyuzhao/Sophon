// pub mod task;
pub mod mem;

use core::marker::PhantomData;
use super::KernelTask;
use proton::task::*;
use proton::kernel_call::KernelCall;


pub struct System {
}

impl System {
    pub fn new() -> Self {
        Self {
        }
    }
}


impl KernelTask for System {
    fn run(&mut self) -> ! {
        log!("Kernel process start");
        loop {
            log!("Kernel process loop");
            // debug_assert!(<K::Arch as AbstractArch>::Interrupt::is_enabled());
            // let m = Message::receive(None);
            // debug!(K: "Kernel received {:?}", m);
            // let kind: KernelCall = unsafe { ::core::mem::transmute(m.kind) };
            // match kind {
            //     KernelCall::MapPhysicalMemory => mem::map_physical_memory::<K>(&m),
            //     _ => {}
            // }
            //     println!("Kernel received {:?}", m);
            //     HANDLERS[m.kind](&m);
            // }
        }
    }
}