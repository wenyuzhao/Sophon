#![allow(incomplete_features)]
#![feature(step_trait)]
#![feature(core_intrinsics)]
#![feature(const_trait_impl)]
#![feature(const_mut_refs)]
#![feature(const_option)]
#![feature(never_type)]
#![feature(format_args_nl)]
#![feature(generic_const_exprs)]
#![no_std]

use address::Address;
use syscall::{ModuleRequest, RawModuleRequest, Syscall};

#[allow(unused)]
#[macro_use]
extern crate log;

pub mod address;
pub mod bitmap_page_allocator;
pub mod cache;
pub mod free_list_allocator;
pub mod page;
pub mod page_table;
pub mod volatile;

pub enum VMRequest {
    Map(usize),
}

impl<'a> ModuleRequest<'a> for VMRequest {
    fn as_raw(&'a self) -> RawModuleRequest<'a> {
        match self {
            Self::Map(size) => RawModuleRequest::new(0, size, &(), &()),
        }
    }
    fn from_raw(raw: RawModuleRequest<'a>) -> Self {
        match raw.id() {
            0 => Self::Map(raw.arg(0)),
            _ => panic!("Unknown request"),
        }
    }
}

pub fn sbrk(size: usize) -> Option<Address> {
    let r = syscall::syscall(Syscall::Sbrk, &[size]);
    if r <= 0 {
        None
    } else {
        Some(Address::new(r as usize))
    }
}
