#![feature(format_args_nl)]
#![feature(default_alloc_error_handler)]
#![no_std]

#[macro_use]
extern crate log;

use kernel_module::{kernel_module, KernelModule};

#[kernel_module]
pub static INIT_FS: InitFS = InitFS;

pub struct InitFS;

impl KernelModule for InitFS {
    fn init(&self) -> anyhow::Result<()> {
        log!("Hello, Kernel Module!");
        Ok(())
    }
}
