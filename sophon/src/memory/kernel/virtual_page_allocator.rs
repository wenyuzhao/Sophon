use super::LOG_KERNEL_HEAP_SIZE;
use crate::utils::unint_lock::UnintMutex;
use core::{iter::Step, ops::Range};
use memory::{address::Address, page::*};

pub static VIRTUAL_PAGE_ALLOCATOR: UnintMutex<VirtualPageAllocator<LOG_KERNEL_HEAP_SIZE>> =
    UnintMutex::new(VirtualPageAllocator::new());

pub const fn table_length(log_heap_size: usize) -> usize {
    (1usize << (log_heap_size - Size4K::LOG_BYTES)) * 4 / 64
}

pub struct VirtualPageAllocator<const LOG_HEAP_SIZE: usize>
where
    [(); table_length(LOG_HEAP_SIZE)]: Sized,
{
    base: Address,
    table: [u64; table_length(LOG_HEAP_SIZE)],
}

const FREE: usize = 0b11;
const USED: usize = 0b00;
const SPLIT: usize = 0b01;

impl<const LOG_HEAP_SIZE: usize> VirtualPageAllocator<LOG_HEAP_SIZE>
where
    [(); table_length(LOG_HEAP_SIZE)]: Sized,
{
    const LOG_UNITS: usize = LOG_HEAP_SIZE - Size4K::LOG_BYTES;
    const LOG_BITS_IN_ENTRY: usize = 6;
    const BITS_IN_ENTRY: usize = 1 << Self::LOG_BITS_IN_ENTRY;

    pub const fn new() -> Self {
        Self {
            base: Address::ZERO,
            table: [0u64; table_length(LOG_HEAP_SIZE)],
        }
    }

    pub fn init(&mut self, base: Address) {
        self.base = base;
        self.set_unit(Self::LOG_UNITS, 0, FREE);
    }

    fn get(&self, sc: usize, i: usize) -> usize {
        let bit_offset = (1usize << (Self::LOG_UNITS - sc)) + (i << 1);
        let mask = 0b11;
        let shift = bit_offset & (Self::BITS_IN_ENTRY - 1);
        let entry = self.table[bit_offset >> Self::LOG_BITS_IN_ENTRY];
        ((entry >> shift) & mask) as usize
    }

    fn set(&mut self, sc: usize, i: usize, v: usize) {
        let bit_offset = (1usize << (Self::LOG_UNITS - sc)) + (i << 1);
        let mask = 0b11;
        let shift = bit_offset & (Self::BITS_IN_ENTRY - 1);
        let entry = self.table[bit_offset >> Self::LOG_BITS_IN_ENTRY];
        let new_entry = ((v as u64) << shift) | (entry & !(mask << shift));
        self.table[bit_offset >> Self::LOG_BITS_IN_ENTRY] = new_entry;
    }

    fn get_unit(&self, size_class: usize, unit: usize) -> usize {
        self.get(size_class, Self::index_in_size_class(size_class, unit))
    }

    fn set_unit(&mut self, size_class: usize, unit: usize, v: usize) {
        self.set(size_class, Self::index_in_size_class(size_class, unit), v)
    }

    const fn size_class(units: usize) -> usize {
        units.next_power_of_two().trailing_zeros() as usize
    }

    const fn entries_in_size_class(size_class: usize) -> usize {
        1usize << (Self::LOG_UNITS - size_class)
    }

    const fn index_in_size_class(size_class: usize, unit: usize) -> usize {
        unit >> size_class
    }

    const fn sibling_unit(size_class: usize, unit: usize) -> usize {
        if (unit >> size_class) & 1 == 0 {
            unit + (1 << size_class)
        } else {
            unit - (1 << size_class)
        }
    }

    const fn parent_unit(size_class: usize, unit: usize) -> usize {
        unit & !((1 << (size_class + 1)) - 1)
    }

    fn search_and_allocate_cell(&mut self, size_class: usize) -> Option<usize> {
        for i in 0..Self::entries_in_size_class(size_class) {
            if self.get(size_class, i) == FREE {
                self.set(size_class, i, USED);
                return Some(i << size_class);
            }
        }
        None
    }

    /// Allocate a power-of-two cell
    fn allocate_cell(&mut self, size_class: usize) -> Option<usize> {
        if size_class > Self::LOG_UNITS {
            return None;
        }
        if let Some(cell) = self.search_and_allocate_cell(size_class) {
            return Some(cell);
        }
        // Split from parent cell
        let parent_size_class = size_class + 1;
        let parent_unit = self.allocate_cell(parent_size_class)?;
        self.set_unit(parent_size_class, parent_unit, SPLIT);
        let child_units = (parent_unit, parent_unit + (1 << size_class));
        self.set_unit(size_class, child_units.1, FREE);
        Some(child_units.0)
    }

    /// Release a power-of-two cell
    fn release_cell(&mut self, size_class: usize, unit: usize) {
        debug_assert_eq!(self.get_unit(size_class, unit), USED);
        self.set_unit(size_class, unit, FREE);
        // Try merge with sibling cell
        if size_class < Self::LOG_UNITS {
            let sibling = Self::sibling_unit(size_class, unit);
            if self.get_unit(size_class, unit) == FREE {
                self.set_unit(size_class, unit, USED);
                self.set_unit(size_class, sibling, USED);
                let parent = Self::parent_unit(size_class, unit);
                let parent_size_class = size_class + 1;
                debug_assert_eq!(self.get_unit(parent_size_class, parent), SPLIT);
                self.set_unit(parent_size_class, parent, USED);
                self.release_cell(parent_size_class, parent)
            }
        }
    }

    pub fn acquire<S: PageSize>(&mut self, pages: usize) -> Range<Page<S>> {
        let small_pages = pages << (S::LOG_BYTES - Size4K::LOG_BYTES);
        let size_class = Self::size_class(small_pages);
        let unit = self.allocate_cell(size_class).unwrap();
        let addr = self.base + (unit << Size4K::LOG_BYTES);

        let page = Page::new(addr);
        page..Page::forward(page, pages)
    }

    pub fn release<S: PageSize>(&self, _pages: Range<Page<S>>) {}
}
