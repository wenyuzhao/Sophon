use core::ops::Range;
use memory::{address::Address, page::*};

pub const LOG_KERNEL_HEAP_SIZE: usize = 38;
pub const KERNEL_HEAP_SIZE: usize = 1 << LOG_KERNEL_HEAP_SIZE;
pub const KERNEL_MEMORY_RANGE: Range<Address> =
    Address::new(0xff0000000000)..Address::new(0xff8000000000);
pub const KERNEL_HEAP_RANGE: Range<Address> =
    Address::new(KERNEL_MEMORY_RANGE.end.as_usize() - KERNEL_HEAP_SIZE)..KERNEL_MEMORY_RANGE.end;

pub const KERNEL_STACK_PAGES: usize = 8;
pub const KERNEL_STACK_SIZE: usize = KERNEL_STACK_PAGES << Size4K::LOG_BYTES;

mod heap;
mod mapper;

pub use heap::{KernelHeapAllocator, KERNEL_HEAP};
pub use mapper::KERNEL_MEMORY_MAPPER;
