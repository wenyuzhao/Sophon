use super::free_list_allocator::FreeListAllocator;
use super::virtual_page_allocator::VIRTUAL_PAGE_ALLOCATOR;
use super::KERNEL_MEMORY_MAPPER;
use crate::memory::physical::PHYSICAL_MEMORY;
use crate::utils::unint_lock::UnintMutex;
use core::alloc::{GlobalAlloc, Layout};
use core::iter::Step;
use core::ops::Range;
use core::usize;
use memory::page::*;
use memory::page_table::{PageFlags, PageFlagsExt};

/// The kernel heap memory manager.
pub struct KernelHeap {
    fa: UnintMutex<FreeListAllocator>,
}

impl KernelHeap {
    const fn new() -> Self {
        Self {
            fa: UnintMutex::new(FreeListAllocator::new()),
        }
    }

    pub fn init(&self) {
        self.fa.lock().init()
    }

    /// Allocate virtual pages that are backed by physical memory.
    pub fn allocate_pages<S: PageSize>(&self, pages: usize) -> Range<Page<S>> {
        let virtual_pages = self.virtual_allocate::<S>(pages);
        for i in 0..pages {
            let frame = PHYSICAL_MEMORY.acquire::<S>().unwrap();
            KERNEL_MEMORY_MAPPER.map(
                Page::forward(virtual_pages.start, i),
                frame,
                PageFlags::kernel_data_flags::<S>(),
            );
        }
        virtual_pages
    }

    /// Release and unmap virtual pages.
    pub fn release_pages<S: PageSize>(&self, pages: Range<Page<S>>) {
        for page in pages {
            let frame = Frame::<S>::new(KERNEL_MEMORY_MAPPER.translate(page.start()).unwrap());
            KERNEL_MEMORY_MAPPER.unmap(page);
            PHYSICAL_MEMORY.release(frame);
        }
    }

    /// Allocate virtual pages that are not backed by any physical memory.
    pub fn virtual_allocate<S: PageSize>(&self, pages: usize) -> Range<Page<S>> {
        VIRTUAL_PAGE_ALLOCATOR.lock().acquire(pages)
    }

    /// Release virtual pages, without updating memory mapping.
    pub fn virtual_release<S: PageSize>(&self, pages: Range<Page<S>>) {
        VIRTUAL_PAGE_ALLOCATOR.lock().release(pages)
    }
}

pub static KERNEL_HEAP: KernelHeap = KernelHeap::new();

/// Rust global allocator that allocate objects into the kernel heap.
pub struct KernelHeapAllocator;

unsafe impl GlobalAlloc for KernelHeapAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let _guard = interrupt::uninterruptable();
        KERNEL_HEAP.fa.lock().alloc(&layout).as_mut_ptr()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let _guard = interrupt::uninterruptable();
        KERNEL_HEAP.fa.lock().free(ptr.into(), &layout)
    }
}
