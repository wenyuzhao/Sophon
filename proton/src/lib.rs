#![feature(asm, llvm_asm)]
#![feature(step_trait)]
#![feature(const_fn)]
#![feature(format_args_nl)]
#![feature(core_intrinsics)]
#![feature(step_trait_ext)]
#![no_std]

#[macro_use]
extern crate bitflags;

#[cfg(feature = "user")]
#[macro_use]
pub mod log;

mod address;
pub mod ipc;
pub mod kernel_call;
pub mod lazy;
pub mod memory;
mod page;
pub mod task;
pub mod utils;

#[cfg(feature = "user")]
#[macro_use]
pub mod driver;

pub use ipc::*;
pub use kernel_call::*;
pub use task::*;
