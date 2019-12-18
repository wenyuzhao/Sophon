pub mod address;
pub mod heap_constants;
pub mod page;
// pub mod frame_allocator;
// pub mod page_table;
// pub mod paging;
pub mod heap;

pub use self::address::*;
pub use self::page::*;
// pub use self::page_table::PageFlags;
// pub use self::paging::*;
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

// pub fn map_user<S: PageSize>(page: Page<S>, frame: Frame<S>, mut flags: PageFlags) -> Page<S> {
//     MemoryManager::map(page, frame, flags);
//     page
// }

// pub fn map_kernel<S: PageSize>(page: Page<S>, frame: Frame<S>, mut flags: PageFlags) {
//     MemoryManager::map(page, frame, flags);
// }

// pub fn update_kernel_page_flags<S: PageSize>(page: Page<S>, mut flags: PageFlags) {
//     MemoryManager::update_flags(page, flags);
// }

// Unmap a kernel page, optionally release its corresponding frame
// pub fn unmap_kernel<S: PageSize>(page: Page<S>, release_frame: bool) {
//     let (frame, _) = MemoryManager::translate(page).unwrap();
//     MemoryManager::unmap(page);
//     if release_frame {
//         MemoryManager::dealloc_frame(frame);
//     }
// }



// use core::ops::*;

// pub struct TemporaryKernelPage<S: PageSize>(Page<S>, bool);

// impl <S: PageSize> Deref for TemporaryKernelPage<S> {
//     type Target = Page<S>;
//     fn deref(&self) -> &Self::Target {
//         &self.0
//     }
// }

// impl <S: PageSize> Drop for TemporaryKernelPage<S> {
//     fn drop(&mut self) {
//         unmap_kernel(self.0, self.1);
//         paging::invalidate_tlb();
//     }
// }


// pub fn map_kernel_temporarily<S: PageSize>(frame: Frame<S>, mut flags: PageFlags, p: Option<usize>) -> TemporaryKernelPage<S> {
//     const MAGIC_PAGE: usize = 0xffff_1234_5600_0000;
//     let page = Page::new(p.unwrap_or(MAGIC_PAGE).into());
//     map_kernel(page, frame, flags);
//     paging::invalidate_tlb();
//     TemporaryKernelPage(page, false)
// }

// pub fn handle_user_pagefault(address: Address) {
//     // let p4 = PageTable::<L4>::get(false);
//     // if let Some((_, flags)) = p4.translate(address) {
//     //     if flags.contains(PageFlags::COPY_ON_WRITE) {
//     //         p4.fix_copy_on_write(address, !flags.contains(PageFlags::SMALL_PAGE));
//     //         return
//     //     }
//     // }
//     unimplemented!()
// }