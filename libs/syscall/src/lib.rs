#![no_std]
#![feature(format_args_nl)]

extern crate alloc;

#[macro_use]
mod log;
mod syscall;

pub use crate::log::UserLogger;
pub use syscall::*;
