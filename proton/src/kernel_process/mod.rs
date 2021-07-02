pub mod system;
pub mod user;


pub trait KernelTask {
    fn run(&mut self) -> !;
}
