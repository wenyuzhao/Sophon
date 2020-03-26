#![feature(asm)]
#![feature(step_trait)]
#![feature(const_fn)]
#![feature(format_args_nl)]
#![no_std]


#[macro_use]
extern crate bitflags;

#[cfg(feature="user")]
#[macro_use]
pub mod log;

pub mod task;
pub mod kernel_call;
pub mod ipc;
mod address;
mod page;
pub mod memory;
pub mod lazy;

#[cfg(feature="user")]
#[macro_use]
pub mod driver;

pub use task::*;
pub use kernel_call::*;
pub use ipc::*;
