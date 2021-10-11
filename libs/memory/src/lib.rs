#![allow(incomplete_features)]
#![feature(step_trait)]
#![feature(core_intrinsics)]
#![feature(const_trait_impl)]
#![feature(const_fn_trait_bound)]
#![feature(const_mut_refs)]
#![feature(const_raw_ptr_to_usize_cast)]
#![feature(const_panic)]
#![feature(const_raw_ptr_deref)]
#![feature(const_option)]
#![feature(never_type)]
#![feature(asm)]
#![feature(format_args_nl)]
#![no_std]

#[macro_use]
extern crate log;

pub mod address;
pub mod page;
pub mod page_table;
