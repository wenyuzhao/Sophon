use crate::user::ipc::Resource;

pub mod user;

pub trait KernelTask {
    fn run(&mut self) -> !;
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

pub struct TestKernelTaskA;

impl KernelTask for TestKernelTaskA {
    fn run(&mut self) -> ! {
        log!("TestKernelTaskA start");
        let resource = Resource::open("system:/test", 0, crate::user::ipc::Mode::ReadOnly).unwrap();
        log!("system:test opened");
        let mut data = [0u8; 4];
        loop {
            resource.read(&mut data).unwrap();
            log!("system:test read -> {:?}", data);
        }
    }
}

pub struct TestKernelTaskB;

impl KernelTask for TestKernelTaskB {
    fn run(&mut self) -> ! {
        log!("TestKernelTaskB start");
        loop {
            log!("TestKernelTaskB loop");
        }
    }
}
