// pub mod task;
pub mod mem;

use super::KernelTask;
use crate::arch::{Arch, TargetArch};
use crate::task::Message;

pub struct System {}

impl System {
    pub fn new() -> Self {
        Self {}
    }
}

impl KernelTask for System {
    fn run(&mut self) -> ! {
        log!("Kernel process start");
        loop {
            debug_assert!(<TargetArch as Arch>::interrupt().is_enabled());
            let m = Message::receive(None);
            log!("Kernel received {:?}", m);
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
pub struct Idle;

impl KernelTask for Idle {
    fn run(&mut self) -> ! {
        loop {
            unsafe {
                asm!("wfe");
            }
        }
    }
}
