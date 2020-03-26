use proton::memory::*;
use crate::AbstractKernel;
use crate::arch::*;

// Allocate a frame and map it to the given virtual address
pub fn memory_map<K: AbstractKernel>(address: Address, size: usize, flags: PageFlags) -> Result<Address, ()> {
    debug_assert!(!flags.contains(PageFlags::PAGE_2M));
    debug_assert!(!flags.contains(PageFlags::PAGE_1G));
    assert!(Page::<Size4K>::is_aligned(address));
    assert!(Page::<Size4K>::is_aligned(size.into()));
    let start_page = Page::<Size4K>::new(address);
    let end_page = Page::<Size4K>::new(address + size);
    for page in start_page..end_page {
        let frame = <K::Arch as AbstractArch>::MemoryManager::alloc_frame();
        <K::Arch as AbstractArch>::MemoryManager::map(page, frame, flags);
        unsafe { page.zero(); }
    }
    Ok(address)
}
