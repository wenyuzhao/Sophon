pub mod system;



pub trait KernelTask {
    fn run(&mut self) -> !;
}
