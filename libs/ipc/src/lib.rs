#![feature(asm)]
#![no_std]

extern crate alloc;

#[macro_use]
pub mod log;
pub mod scheme;
pub mod syscall;
mod uri;
