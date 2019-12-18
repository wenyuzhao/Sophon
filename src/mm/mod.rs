pub mod address;
pub mod heap_constants;
pub mod page;
pub mod heap;

pub use self::address::*;
pub use self::page::*;

use crate::arch::*;
use Target::MemoryManager;

bitflags! {
    pub struct PageFlags: usize {
        const PRESENT     = 1 << 0;
        const ACCESSED    = 1 << 1;
        const KERNEL      = 1 << 2;
        const NO_WRITE    = 1 << 3;
        const NO_EXEC     = 1 << 4;
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

/// Allocate a frame and map it to the given virtual address
pub fn memory_map(address: Address, size: usize, mut flags: PageFlags) -> Result<Address, ()> {
    assert!(Page::<Size4K>::is_aligned(address));
    assert!(Page::<Size4K>::is_aligned(size.into()));
    let start_page = Page::<Size4K>::new(address);
    let end_page = Page::<Size4K>::new(address + size);
    for page in start_page..end_page {
        let frame = MemoryManager::alloc_frame();
        MemoryManager::map(page, frame, flags);
        unsafe { page.zero(); }
    }
    Ok(address)
}
