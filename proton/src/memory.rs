use core::ops::Add;

use spin::Mutex;

pub use crate::address::*;
pub use crate::page::*;

bitflags! {
    pub struct PageFlags: usize {
        const PAGE_4K     = 0b00 << 0;
        const PAGE_2M     = 0b01 << 0;
        const PAGE_1G     = 0b10 << 0;
        const PRESENT     = 0b1 << 2;
        const ACCESSED    = 0b1 << 3;
        const KERNEL      = 0b1 << 4;
        const NO_WRITE    = 0b1 << 5;
        const NO_EXEC     = 0b1 << 6;
        const NO_CACHE    = 0b1 << 7;
    }
}

impl PageFlags {
    pub fn user_stack_flags() -> Self {
        Self::PRESENT | Self::ACCESSED | Self::NO_EXEC
    }
    pub fn user_code_flags() -> Self {
        Self::PRESENT | Self::ACCESSED// | Self::NO_WRITE
    }
}

pub struct PhysicalMemoryManager {
    table: Option<&'static [u8]>,
}

impl PhysicalMemoryManager {
    pub const fn new() -> Self {
        Self {
            table: None
        }
    }

    // pub fn init(total_pages: usize, available_memory: impl Iterator<Item=(Address, usize)>) {
    //     let log_total_pages =
    //     for (start, pages) in available_memory {
    //         self
    //     }
    // }

    // pub fn alloc_fixed(&mut self, addr: Address, pages: usize) -> Address {

    // }

    // pub fn alloc(log_pages: usize) -> Address {

    // }
}