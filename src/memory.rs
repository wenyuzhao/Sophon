pub use crate::utils::address::*;
pub use crate::utils::page::*;

use crate::arch::*;
use Target::MemoryManager;

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
    debug_assert!(!flags.contains(PageFlags::PAGE_2M));
    debug_assert!(!flags.contains(PageFlags::PAGE_1G));
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
