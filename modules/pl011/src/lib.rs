#![feature(format_args_nl)]
#![feature(default_alloc_error_handler)]
#![no_std]

#[macro_use]
extern crate log;

use kernel_module::{kernel_module, KernelModule};

#[kernel_module]
pub static PL011_MODULE: PL011 = PL011;

pub struct PL011;

impl KernelModule for PL011 {
    fn init(&self) -> anyhow::Result<()> {
        log!("Hello, Kernel Module!");
        Ok(())
    }
}
