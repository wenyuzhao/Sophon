pub mod address;
pub mod heap_constants;
pub mod page;
pub mod frame_allocator;
pub mod page_table;
pub mod paging;
pub mod heap;

pub use self::address::*;
pub use self::page::*;
pub use self::page_table::*;
pub use self::paging::*;


pub fn map_user<S: PageSize>(page: Page<S>, frame: Frame<S>, mut flags: PageFlags) -> Page<S> {
    if S::LOG_SIZE == Size4K::LOG_SIZE {
        flags |= PageFlags::SMALL_PAGE;
    }
    let p4 = PageTable::<L4>::get(false);
    p4.map(page, frame, flags);
    page
}

pub fn map_kernel<S: PageSize>(page: Page<S>, frame: Frame<S>, mut flags: PageFlags) {
    if S::LOG_SIZE == Size4K::LOG_SIZE {
        flags |= PageFlags::SMALL_PAGE;
    }
    let p4 = PageTable::<L4>::get(true);
    p4.map(page, frame, flags);
}

pub fn unmap_kernel<S: PageSize>(page: Page<S>, release_frame: bool) {
    let p4 = PageTable::<L4>::get(true);
    let frame = Frame::<S>::new(p4.translate(page.start()).unwrap().0);
    p4.unmap(page);
    if release_frame {
        frame_allocator::free(frame);
    }
}



use core::ops::*;

pub struct TemporaryKernelPage<S: PageSize>(Page<S>, bool);

impl <S: PageSize> Deref for TemporaryKernelPage<S> {
    type Target = Page<S>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl <S: PageSize> Drop for TemporaryKernelPage<S> {
    fn drop(&mut self) {
        // paging::invalidate_tlb();
        unmap_kernel(self.0, self.1);
        paging::invalidate_tlb();
    }
}

pub fn map_kernel_temporarily<S: PageSize>(frame: Frame<S>, mut flags: PageFlags) -> TemporaryKernelPage<S> {
    const MAGIC_PAGE: usize = 0xffff_1234_5600_0000;
    let page = Page::new(MAGIC_PAGE.into());
    // paging::invalidate_tlb();
    map_kernel(page, frame, flags);
    paging::invalidate_tlb();
    TemporaryKernelPage(page, false)
}

pub fn map_kernel_temporarily2<S: PageSize>(frame: Frame<S>, mut flags: PageFlags, p: Option<usize>) -> TemporaryKernelPage<S> {
    const MAGIC_PAGE: usize = 0xffff_1234_5600_0000;
    let page = Page::new(p.unwrap_or(MAGIC_PAGE).into());
    // paging::invalidate_tlb();
    map_kernel(page, frame, flags);
    paging::invalidate_tlb();
    TemporaryKernelPage(page, false)
}
