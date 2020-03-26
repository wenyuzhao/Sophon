use proton_kernel::kernel_process::KernelTask;



pub struct Idle;

impl KernelTask for Idle {
    fn run(&mut self) -> ! {
        loop {
            unsafe { asm!("wfe"); }
        }
    }
}