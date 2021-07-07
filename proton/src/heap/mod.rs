pub mod constants;

use crate::utils::address::*;
use crate::utils::page::*;
use core::alloc::{GlobalAlloc, Layout};
use core::cmp::{max, min};
use spin::Mutex;

use crate::{arch::*, memory::physical::*};

const MIN_SIZE: usize = 1 << 3;

pub struct FreeListAllocator {
    start: Address,
    end: Address,
    cells: [Address; 28], // (1<<3), (1<<4), (1<<5), ..., (1<<30)
}

impl FreeListAllocator {
    const fn new() -> Self {
        Self {
            start: Address::ZERO,
            end: Address::ZERO,
            cells: [Address::ZERO; 28],
        }
    }

    fn dump(&self) {
        log!("Heap: {:?}..{:?}", self.start, self.end);
    }

    fn init(&mut self) {
        // Allocate 128M as kernel heap
        let heap = PHYSICAL_PAGE_RESOURCE
            .lock()
            .acquire::<Size2M>(128)
            .unwrap();
        let heap_start: Address = Address::<V>::from(heap.start.start().as_usize());
        let heap_limit: Address = Address::<V>::from(heap.end.start().as_usize());
        self.start = heap_start;
        self.end = heap_limit;
        // println!("Heap: {:?}..{:?}", heap_start, heap_limit);
        let mut cursor = heap_start;
        while cursor < heap_limit {
            let align = cursor.as_usize().trailing_zeros();
            let size = min(1 << align, heap_limit - cursor);
            assert!(size > 0);
            let size_class = Self::size_class(size);
            assert!(cursor.as_usize() & ((1 << (size_class + 3)) - 1) == 0);
            self.push_cell(size_class, cursor);
            cursor += size;
        }
        // // Allocate 256M as kernel heap
        // let heap = PHYSICAL_PAGE_RESOURCE.lock().acquire::<Size2M>(constants::KERNEL_HEAP_LARGE_PAGES).unwrap();
        // let heap_start_frame = Frame::<Size2M>::new(heap.start.start());
        // let heap_end_frame = Frame::<Size2M>::new(heap.end.start());
        // // Map physical memory
        // let virtual_memory = constants::kernel_heap_memory();
        // let mut current_frame = heap_start_frame;
        // for page in virtual_memory {
        //     // map page -> current_frame
        //     proton::memory::
        //     current_frame.memory
        // }

        // // println!("Heap: {:?}..{:?}", heap_start, heap_limit);
        // let mut cursor = heap_start;
        // while cursor < heap_limit {
        //     let align = cursor.as_usize().trailing_zeros();
        //     let size = min(1 << align, heap_limit - cursor);
        //     assert!(size > 0);
        //     let size_class = Self::size_class(size);
        //     assert!(cursor.as_usize() & ((1 << (size_class + 3)) - 1) == 0);
        //     self.push_cell(size_class, cursor);
        //     cursor += size;
        // }
    }

    fn push_cell(&mut self, size_class: usize, cell: Address) {
        unsafe {
            cell.store(self.cells[size_class]);
        }
        self.cells[size_class] = cell;
    }

    fn size_class(block_size: usize) -> usize {
        let mut class = 0;
        while block_size > (MIN_SIZE << class) {
            class += 1;
        }
        class
    }

    fn cell_size(layout: &Layout) -> usize {
        max(
            layout.size().next_power_of_two(),
            max(layout.align(), MIN_SIZE),
        )
    }

    fn alloc_cell(&mut self, size_class: usize) -> Option<Address> {
        if size_class >= self.cells.len() {
            None
        } else if !self.cells[size_class].is_zero() {
            let cell = self.cells[size_class];
            self.cells[size_class] = unsafe { cell.load() };
            unsafe { cell.store(Address::<V>::ZERO) };
            Some(cell)
        } else {
            let next_level_cell = self.alloc_cell(size_class + 1)?;
            let (cell0, cell1) = (next_level_cell, next_level_cell + (MIN_SIZE << size_class));
            self.push_cell(size_class, cell1);
            Some(cell0)
        }
    }

    fn zero(start: Address, size: usize) {
        let (mut cursor, limit) = (start, start + size);
        while cursor < limit {
            unsafe {
                cursor.store(0usize);
            }
            cursor = cursor + ::core::mem::size_of::<usize>();
        }
    }

    fn alloc(&mut self, layout: &Layout) -> Address {
        let cell_size = Self::cell_size(&layout);
        let size_class = Self::size_class(cell_size);

        match self.alloc_cell(size_class) {
            Some(cell) => {
                Self::zero(cell, cell_size);
                cell
            }
            None => panic!("OutOfMemory"),
        }
    }

    fn free(&mut self, start: Address, layout: &Layout) {
        let cell_size = Self::cell_size(&layout);
        let size_class = Self::size_class(cell_size);
        self.push_cell(size_class, start);
    }
}

/// FIXME: Bad performance!
pub struct GlobalAllocator {
    fa: Mutex<FreeListAllocator>,
}

impl GlobalAllocator {
    pub const fn new() -> Self {
        Self {
            fa: Mutex::new(FreeListAllocator::new()),
        }
    }

    pub fn init(&self) {
        self.fa.lock().init()
    }

    pub fn dump(&self) {
        self.fa.lock().dump()
    }
}

unsafe impl GlobalAlloc for GlobalAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let a = TargetArch::uninterruptable(|| self.fa.lock().alloc(&layout).as_mut_ptr());
        a
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        TargetArch::uninterruptable(|| self.fa.lock().free(ptr.into(), &layout));
    }
}

// pub struct KernelHeap {

// }

// impl AbstractKernelHeap for KernelHeap {
//     // const RANGE: (Address, Address) = constants::kernel_heap_start()
//     fn init() {
//         crate::ALLOCATOR.init()
//     }
// }
