#![allow(incomplete_features)]
#![feature(const_fn)]
#![feature(format_args_nl)]
#![feature(associated_type_defaults)]
#![feature(box_syntax)]
#![feature(core_intrinsics)]
#![feature(never_type)]
#![feature(const_fn_transmute)]
#![feature(const_raw_ptr_deref)]
#![feature(const_panic)]
#![feature(specialization)]
#![feature(const_mut_refs)]
#![feature(impl_trait_in_bindings)]
#![feature(min_type_alias_impl_trait)]
#![feature(step_trait)]
#![feature(step_trait_ext)]
#![feature(global_asm)]
#![feature(asm)]
#![feature(const_impl_trait)]
#![feature(const_fn_fn_ptr_basics)]
#![feature(const_trait_impl)]
#![feature(const_raw_ptr_to_usize_cast)]
#![feature(const_generics)]
#![feature(const_maybe_uninit_assume_init)]
#![no_std]

use core::ops::Range;

use utils::page::Frame;

extern crate alloc;
extern crate elf_rs;

#[macro_use]
pub mod utils;
#[macro_use]
pub mod log;
pub mod arch;
pub mod boot_driver;
pub mod heap;
pub mod kernel_tasks;
pub mod memory;
// pub mod page_table;
pub mod scheduler;
pub mod task;

pub struct BootInfo {
    pub available_physical_memory: &'static [Range<Frame>],
    pub device_tree: &'static [u8],
}
