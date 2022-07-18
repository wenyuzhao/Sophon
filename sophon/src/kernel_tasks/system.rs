use super::KernelTask;
use alloc::vec;
use core::arch::asm;
use ipc::{
    scheme::{Args, Mode, Resource},
    ProcId,
};

pub struct System;

impl System {
    fn spawn_user_process(file: &str) -> ProcId {
        let resource = Resource::open("proc:/spawn", 0, Mode::ReadOnly).unwrap();
        let mut proc_id: ProcId = ProcId::NULL;
        resource.write(Args::new((file, &mut proc_id))).unwrap();
        proc_id
    }
    fn load_kernel_module(name: &str, file: &str) {
        log!("Loading kernel module: {}", file);
        let mut data = vec![];
        let resource = Resource::open(file, 0, Mode::ReadOnly).unwrap();
        log!("Resource opened");
        loop {
            let mut buf = [0u8; 4096];
            let len = resource.read(&mut buf).unwrap();
            if len == 0 {
                break;
            }
            data.extend_from_slice(&buf[..len]);
        }
        log!("Resource loaded");
        crate::modules::register(name, data)
    }
}

impl KernelTask for System {
    fn run(&mut self) -> ! {
        Self::load_kernel_module("hello", "init:/libhello.so");
        Self::load_kernel_module("vfs", "init:/libvfs.so");
        Self::spawn_user_process("init:/init");
        loop {
            unsafe {
                asm!("wfe");
            }
        }
    }
}
