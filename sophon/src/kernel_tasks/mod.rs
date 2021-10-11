pub mod system;
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
        loop {}
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
