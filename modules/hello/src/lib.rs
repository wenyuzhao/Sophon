#![feature(format_args_nl)]
#![feature(default_alloc_error_handler)]
#![no_std]

#[macro_use]
extern crate kernel_module;
extern crate alloc;

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

#[test]
fn simple_test() {
    assert_eq!(1 + 1, 2);
}

#[test]
fn alloc_test() {
    let mut array = alloc::vec![0usize; 0];
    for v in 1..=100 {
        array.push(v);
    }
    let sum: usize = array.iter().sum();
    assert_eq!(sum, (1 + 100) * 100 / 2);
}
