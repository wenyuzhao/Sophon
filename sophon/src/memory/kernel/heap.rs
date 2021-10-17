use super::{KERNEL_HEAP_RANGE, KERNEL_MEMORY_MAPPER, LOG_KERNEL_HEAP_SIZE};
use crate::memory::physical::PHYSICAL_MEMORY;
use crate::utils::unint_lock::UnintMutex;
use core::alloc::{GlobalAlloc, Layout};
use core::iter::Step;
use core::ops::Range;
use core::usize;
use memory::address::V;
use memory::bitmap_page_allocator::BitMapPageAllocator;
use memory::free_list_allocator::FreeListAllocator;
use memory::page::*;
use memory::page_table::{PageFlags, PageFlagsExt};

static VIRTUAL_PAGE_ALLOCATOR: UnintMutex<BitMapPageAllocator<V, LOG_KERNEL_HEAP_SIZE>> =
    UnintMutex::new(BitMapPageAllocator::new());

pub static KERNEL_HEAP: KernelHeap = KernelHeap::new();

/// The kernel heap memory manager.
pub struct KernelHeap {
    fa: UnintMutex<FreeListAllocator<V, Self, LOG_KERNEL_HEAP_SIZE>>,
}

impl KernelHeap {
    const fn new() -> Self {
        Self {
            fa: UnintMutex::new(FreeListAllocator::new()),
        }
    }

    pub fn init(&'static self) {
        VIRTUAL_PAGE_ALLOCATOR.lock().init(KERNEL_HEAP_RANGE.start);
        self.fa.lock().init(self)
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

impl PageResource<V> for KernelHeap {
    /// Allocate virtual pages that are backed by physical memory.
    fn acquire_pages<S: PageSize>(&self, pages: usize) -> Option<Range<Page<S>>> {
        let virtual_pages = KERNEL_HEAP.virtual_allocate::<S>(pages);
        for i in 0..pages {
            let frame = PHYSICAL_MEMORY.acquire::<S>().unwrap();
            KERNEL_MEMORY_MAPPER.map(
                Page::forward(virtual_pages.start, i),
                frame,
                PageFlags::kernel_data_flags::<S>(),
            );
        }
        Some(virtual_pages)
    }

    /// Release and unmap virtual pages.
    fn release_pages<S: PageSize>(&self, pages: Range<Page<S>>) {
        for page in pages {
            let frame = Frame::<S>::new(KERNEL_MEMORY_MAPPER.translate(page.start()).unwrap());
            KERNEL_MEMORY_MAPPER.unmap(page);
            PHYSICAL_MEMORY.release(frame);
        }
    }
}

/// Rust global allocator that allocate objects into the kernel heap.
pub struct KernelHeapAllocator;

unsafe impl GlobalAlloc for KernelHeapAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let _guard = interrupt::uninterruptible();
        KERNEL_HEAP.fa.lock().alloc(&layout).as_mut_ptr()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let _guard = interrupt::uninterruptible();
        KERNEL_HEAP.fa.lock().free(ptr.into(), &layout)
    }
}
