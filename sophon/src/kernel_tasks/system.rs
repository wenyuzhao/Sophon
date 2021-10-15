use ipc::{
    scheme::{Mode, Resource},
    ProcId,
};

use super::KernelTask;

pub struct System;

impl System {
    fn spawn_user_process(file: &str) -> ProcId {
        let resource = Resource::open("proc:/spawn", 0, Mode::ReadOnly).unwrap();
        let mut proc_id: ProcId = ProcId::NULL;
        resource.write_any((file, &mut proc_id)).unwrap();
        proc_id
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
