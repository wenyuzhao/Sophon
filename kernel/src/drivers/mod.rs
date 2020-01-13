/// System drivers, running in kernel space


pub trait KernelDriver {
    fn init(&self);
}

pub mod fat;
pub mod emmc;