use core::ops::Range;
use memory::{address::Address, page::*};

pub const KERNEL_MEMORY_RANGE: Range<Address> =
    Address::new(0xff0000000000)..Address::new(0xff8000000000);
pub const KERNEL_HEAP_RANGE: Range<Address> = Address::new(0xff4000000000)..KERNEL_MEMORY_RANGE.end;
pub const KERNEL_HEAP_SIZE: usize = KERNEL_HEAP_RANGE.end - KERNEL_HEAP_RANGE.start;

pub const KERNEL_STACK_PAGES: usize = 8;
pub const KERNEL_STACK_SIZE: usize = KERNEL_STACK_PAGES << Size4K::LOG_BYTES;

mod heap;
mod mapper;

pub use heap::{KernelHeapAllocator, KERNEL_HEAP};
pub use mapper::KERNEL_MEMORY_MAPPER;
