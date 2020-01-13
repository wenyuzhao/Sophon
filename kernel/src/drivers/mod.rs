/// System drivers, running in kernel space


pub trait KernelDriver {
    fn init(&self);
}

pub mod fat;
pub mod sd_card;
pub mod emmc;