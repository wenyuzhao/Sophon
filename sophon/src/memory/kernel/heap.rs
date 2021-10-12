use super::KERNEL_HEAP_RANGE;
use super::KERNEL_HEAP_SIZE;
use super::KERNEL_MEMORY_MAPPER;
use crate::memory::physical::PHYSICAL_MEMORY;
use core::alloc::{GlobalAlloc, Layout};
use core::cmp::{max, min};
use core::iter::Step;
use core::ops::Range;
use core::usize;
use memory::address::*;
use memory::page::*;
use memory::page_table::{PageFlags, PageFlagsExt};
use spin::Mutex;

static VIRTUAL_PAGE_ALLOCATOR: Mutex<VirtualPageAllocator> =
    Mutex::new(VirtualPageAllocator::new());

struct VirtualPageAllocator {
    table_4k: [u64; KERNEL_HEAP_SIZE / Size4K::BYTES / 64],
    table_2m: [u64; KERNEL_HEAP_SIZE / Size2M::BYTES / 64],
    table_1g: [u64; KERNEL_HEAP_SIZE / Size1G::BYTES / 64],
}

impl VirtualPageAllocator {
    const LOG_BITS_IN_ENTRY: usize = 6;
    const BITS_IN_ENTRY: usize = 1 << Self::LOG_BITS_IN_ENTRY;

    const fn new() -> Self {
        Self {
            table_4k: [0u64; KERNEL_HEAP_SIZE / Size4K::BYTES / 64],
            table_2m: [0u64; KERNEL_HEAP_SIZE / Size2M::BYTES / 64],
            table_1g: [0u64; KERNEL_HEAP_SIZE / Size1G::BYTES / 64],
        }
    }

    fn get(table: &mut [u64], i: usize) -> bool {
        table[i >> Self::LOG_BITS_IN_ENTRY] & (1 << (i & (Self::BITS_IN_ENTRY - 1))) != 0
    }
    fn set(table: &mut [u64], i: usize, v: bool) {
        if v {
            table[i >> Self::LOG_BITS_IN_ENTRY] |= 1 << (i & (Self::BITS_IN_ENTRY - 1));
        } else {
            table[i >> Self::LOG_BITS_IN_ENTRY] &= !(1 << (i & (Self::BITS_IN_ENTRY - 1)));
        }
    }
    fn search_and_mark(table: &mut [u64], units: usize) -> Option<usize> {
        // FIXME: performance
        let mut i = 0;
        while i < table.len() << Self::LOG_BITS_IN_ENTRY {
            if !Self::get(table, i) {
                let start = i;
                let mut span = 0;
                while span < units && !Self::get(table, i) {
                    i += 1;
                    span += 1;
                }
                if span == units {
                    Self::mark(table, start..(start + span));
                    return Some(start);
                }
            }
            i += 1;
        }
        None
    }

    fn mark(table: &mut [u64], range: Range<usize>) {
        for i in range {
            Self::set(table, i, true);
        }
    }

    fn acquire<S: PageSize>(&mut self, pages: usize) -> Range<Page<S>> {
        let start_index = if S::BYTES == Size4K::BYTES {
            let start_index = Self::search_and_mark(&mut self.table_4k, pages).unwrap();
            let range_4k = start_index..start_index + pages;
            Self::mark(
                &mut self.table_2m,
                (range_4k.start >> (Size2M::LOG_BYTES - Size4K::LOG_BYTES))
                    ..((range_4k.end - 1) >> (Size2M::LOG_BYTES - Size4K::LOG_BYTES)) + 1,
            );
            Self::mark(
                &mut self.table_1g,
                (range_4k.start >> (Size1G::LOG_BYTES - Size4K::LOG_BYTES))
                    ..((range_4k.end - 1) >> (Size1G::LOG_BYTES - Size4K::LOG_BYTES)) + 1,
            );
            start_index
        } else if S::BYTES == Size2M::BYTES {
            let start_index = Self::search_and_mark(&mut self.table_2m, pages).unwrap();
            let range_2m = start_index..start_index + pages;
            Self::mark(
                &mut self.table_4k,
                (range_2m.start << (Size2M::LOG_BYTES - Size4K::LOG_BYTES))
                    ..(range_2m.end << (Size2M::LOG_BYTES - Size4K::LOG_BYTES)),
            );
            Self::mark(
                &mut self.table_1g,
                (range_2m.start >> (Size1G::LOG_BYTES - Size2M::LOG_BYTES))
                    ..((range_2m.end - 1) >> (Size1G::LOG_BYTES - Size2M::LOG_BYTES)) + 1,
            );
            start_index
        } else {
            let start_index = Self::search_and_mark(&mut self.table_1g, pages).unwrap();
            let range_1g = start_index..start_index + pages;
            Self::mark(
                &mut self.table_4k,
                (range_1g.start << (Size1G::LOG_BYTES - Size4K::LOG_BYTES))
                    ..(range_1g.end << (Size1G::LOG_BYTES - Size4K::LOG_BYTES)),
            );
            Self::mark(
                &mut self.table_2m,
                (range_1g.start << (Size1G::LOG_BYTES - Size2M::LOG_BYTES))
                    ..(range_1g.end << (Size1G::LOG_BYTES - Size2M::LOG_BYTES)),
            );
            start_index
        };

        let page = Page::<S>::new(KERNEL_HEAP_RANGE.start + (start_index << S::LOG_BYTES));
        page..Page::forward(page, pages)
    }

    fn release<S: PageSize>(&self, _pages: Range<Page<S>>) {}
}

pub struct FreeListAllocator {
    cells: [Address; KERNEL_HEAP_SIZE.trailing_zeros() as usize + 1],
    retry: bool,
}

impl FreeListAllocator {
    const MIN_SIZE: usize = 1 << 4;

    const fn new() -> Self {
        Self {
            cells: [Address::ZERO; KERNEL_HEAP_SIZE.trailing_zeros() as usize + 1],
            retry: false,
        }
    }

    fn init(&mut self) {}

    #[inline(always)]
    fn pop_cell(&mut self, size_class: usize) -> Option<Address> {
        let cell = self.cells[size_class];
        if cell.is_zero() {
            None
        } else {
            self.cells[size_class] = unsafe { cell.load() };
            Some(cell)
        }
    }

    #[inline(always)]
    fn push_cell(&mut self, size_class: usize, cell: Address) {
        unsafe {
            cell.store(self.cells[size_class]);
        }
        self.cells[size_class] = cell;
    }

    const fn size_class(block_size: usize) -> usize {
        block_size.next_power_of_two().trailing_zeros() as _
    }

    #[inline(always)]
    fn cell_size(layout: &Layout) -> usize {
        max(layout.pad_to_align().size(), Self::MIN_SIZE)
    }

    fn release_large_pages(&mut self) {
        let start_sc = Self::size_class(Size2M::BYTES);
        let mut sc = start_sc;
        while sc < self.cells.len() {
            while let Some(cell) = self.pop_cell(sc) {
                let pages = 1usize << sc >> Size2M::LOG_BYTES;
                let start = Page::<Size2M>::new(cell);
                KERNEL_HEAP.release_pages(start..Page::forward(start, pages));
            }
            sc += 1;
        }
    }

    fn alloc_cell(&mut self, size_class: usize) -> Option<Address> {
        if size_class >= self.cells.len() {
            None
        } else if let Some(cell) = self.pop_cell(size_class) {
            Some(cell)
        } else {
            let next_level_cell = self.alloc_cell(size_class + 1)?;
            let (cell0, cell1) = (next_level_cell, next_level_cell + (1 << size_class));
            self.push_cell(size_class, cell1);
            Some(cell0)
        }
    }

    #[inline(always)]
    fn alloc_cell_fast(&mut self, size_class: usize) -> Option<Address> {
        if let Some(cell) = self.pop_cell(size_class) {
            Some(cell)
        } else {
            None
        }
    }

    fn alloc_cell_slow(&mut self, size_class: usize) -> Address {
        match self.alloc_cell(size_class) {
            Some(cell) => cell,
            None => {
                assert!(!self.retry, "OutOfMemory");
                let pages = (((1 << size_class) + Size2M::MASK) >> Size2M::LOG_BYTES) << 1;
                let vs = KERNEL_HEAP.allocate_pages::<Size2M>(pages);
                let mut cursor = vs.start.start();
                let end = vs.end.start();
                while cursor < end {
                    let align = cursor.as_usize().trailing_zeros();
                    let size = min(1 << align, end - cursor);
                    assert!(size > 0);
                    let size_class = if size.is_power_of_two() {
                        Self::size_class(size)
                    } else {
                        Self::size_class(size) - 1
                    };
                    assert!(cursor.as_usize() & ((1 << size_class) - 1) == 0);
                    self.push_cell(size_class, cursor);
                    cursor += size;
                }
                self.retry = true;
                let x = self.alloc_cell_slow(size_class);
                self.retry = false;
                x
            }
        }
    }

    #[inline(always)]
    fn alloc(&mut self, layout: &Layout) -> Address {
        let cell_size = Self::cell_size(&layout);
        let size_class = Self::size_class(cell_size);
        if let Some(cell) = self.alloc_cell_fast(size_class) {
            return cell;
        }
        self.alloc_cell_slow(size_class)
    }

    #[inline(always)]
    fn free(&mut self, start: Address, layout: &Layout) {
        let cell_size = Self::cell_size(&layout);
        let size_class = Self::size_class(cell_size);
        self.push_cell(size_class, start);
        self.release_large_pages();
    }
}

pub struct KernelHeap {
    fa: Mutex<FreeListAllocator>,
}

impl KernelHeap {
    const fn new() -> Self {
        Self {
            fa: Mutex::new(FreeListAllocator::new()),
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
