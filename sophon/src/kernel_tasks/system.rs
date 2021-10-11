use alloc::vec;
use ipc::{
    scheme::{Mode, Resource},
    ProcId,
};

use crate::{kernel_tasks::user::UserTask, task::Proc};

use super::KernelTask;

pub struct System;

impl System {
    fn spawn_user_process(file: &str) -> ProcId {
        let mut data = vec![];
        let resource = Resource::open(file, 0, Mode::ReadOnly).unwrap();
        loop {
            let mut buf = [0u8; 4096];
            let len = resource.read(&mut buf).unwrap();
            if len == 0 {
                break;
            }
            data.extend_from_slice(&buf[..len]);
        }
        Proc::spawn(box UserTask::new(data)).id
    }
}

impl KernelTask for System {
    fn run(&mut self) -> ! {
        Self::spawn_user_process("init:/scheme_test");
        Self::spawn_user_process("init:/init");
        loop {
            unsafe {
                asm!("wfe");
            }
        }
    }
}
