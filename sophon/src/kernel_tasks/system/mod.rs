// pub mod task;
pub mod mem;

use super::KernelTask;
use crate::task::uri::Uri;
use crate::user::ipc::{Resource, Result as IoResult, SchemeServer};

pub struct System {}

impl System {
    pub fn new() -> Self {
        Self {}
    }
}

impl KernelTask for System {
    fn run(&mut self) -> ! {
        log!("Kernel process start");
        SystemSchemeServer {}.register();
        // loop {
        //     debug_assert!(<TargetArch as Arch>::Interrupt::is_enabled());
        //     let m = Message::receive(None);
        //     log!("Kernel received {:?}", m);
        //     m.reply(m.get_data::<usize>() + 1)
        //     // let kind: KernelCall = unsafe { ::core::mem::transmute(m.kind) };
        //     // match kind {
        //     //     KernelCall::MapPhysicalMemory => mem::map_physical_memory::<K>(&m),
        //     //     _ => {}
        //     // }
        //     //     println!("Kernel received {:?}", m);
        //     //     HANDLERS[m.kind](&m);
        //     // }
        // }
    }
}

struct SystemSchemeServer {}

impl SchemeServer for SystemSchemeServer {
    fn open(&self, _uri: &Uri) -> IoResult<Resource> {
        log!("SystemSchemeServer 0");
        Ok(Resource(0))
    }
    fn read(&self, _fd: Resource, buf: &mut [u8]) -> IoResult<()> {
        buf[0] += 1;
        Ok(())
    }
    fn write(&self, _fd: Resource, _buf: &[u8]) -> IoResult<()> {
        unimplemented!()
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
