#![allow(incomplete_features)]
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
#![feature(global_asm)]
#![feature(asm)]
#![feature(const_impl_trait)]
#![feature(const_fn_fn_ptr_basics)]
#![feature(const_trait_impl)]
#![feature(const_generics)]
#![feature(const_maybe_uninit_assume_init)]
#![feature(allocator_api)]
#![feature(const_fn_trait_bound)]
#![feature(const_raw_ptr_to_usize_cast)]
#![feature(const_option)]
#![feature(const_btree_new)]
#![no_std]

use core::ops::Range;
use utils::{address::Address, page::Frame};

extern crate alloc;
extern crate elf_rs;

#[macro_use]
pub mod utils;
#[macro_use]
pub mod log;
pub mod arch;
pub mod boot_driver;
#[path = "../init-fs.rs"]
pub mod initfs;
pub mod kernel_tasks;
pub mod memory;
pub mod scheme;
pub mod task;

pub struct BootInfo {
    pub available_physical_memory: &'static [Range<Frame>],
    pub device_tree: &'static [u8],
    pub init_fs: &'static [u8],
    pub uart: Option<Address>,
}
