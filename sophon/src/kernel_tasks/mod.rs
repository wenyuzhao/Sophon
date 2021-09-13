use crate::task::uri::Uri;

pub mod system;
pub mod user;

pub trait KernelTask {
    fn run(&mut self) -> !;
}

pub struct TestKernelTaskA;

impl KernelTask for TestKernelTaskA {
    fn run(&mut self) -> ! {
        log!("TestKernelTaskA start");
        let resource = Uri::open("system:/test").unwrap();
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
