#![feature(format_args_nl)]
#![feature(default_alloc_error_handler)]
#![no_std]

#[macro_use]
extern crate log;

use kernel_module::{kernel_module, KernelModule};

#[kernel_module]
pub static HELLO: Hello = Hello;

pub struct Hello;

impl KernelModule for Hello {
    fn init(&mut self) -> anyhow::Result<()> {
        log!("Hello, Kernel Module!");
        Ok(())
    }
}
