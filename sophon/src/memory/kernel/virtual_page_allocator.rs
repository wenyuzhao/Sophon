use super::LOG_KERNEL_HEAP_SIZE;
use crate::utils::unint_lock::UnintMutex;
use core::{iter::Step, ops::Range};
use memory::{address::Address, page::*};

pub static VIRTUAL_PAGE_ALLOCATOR: UnintMutex<VirtualPageAllocator<LOG_KERNEL_HEAP_SIZE>> =
    UnintMutex::new(VirtualPageAllocator::new());

#[doc(hidden)]
pub const fn table_length(log_heap_size: usize) -> usize {
    (1usize << (log_heap_size - Size4K::LOG_BYTES)) * 2 / core::mem::size_of::<Word>()
}

const FREE: usize = 0b1;
const USED: usize = 0b0;
const MASK: u64 = 0b1;
type Word = u64;

pub struct VirtualPageAllocator<const LOG_HEAP_SIZE: usize>
where
    [(); table_length(LOG_HEAP_SIZE)]: Sized,
{
    base: Address,
    table: [Word; table_length(LOG_HEAP_SIZE)],
}

impl<const LOG_HEAP_SIZE: usize> VirtualPageAllocator<LOG_HEAP_SIZE>
where
    [(); table_length(LOG_HEAP_SIZE)]: Sized,
{
    const LOG_UNITS: usize = LOG_HEAP_SIZE - Size4K::LOG_BYTES;
    const LOG_BITS_IN_WORD: usize = (core::mem::size_of::<Word>() * 8).trailing_zeros() as usize;
    const BITS_IN_WORD: usize = 1 << Self::LOG_BITS_IN_WORD;

    pub const fn new() -> Self {
        Self {
            base: Address::ZERO,
            table: [0; table_length(LOG_HEAP_SIZE)],
        }
    }

    pub fn init(&mut self, base: Address) {
        self.base = base;
        self.set_unit(Self::LOG_UNITS, 0, FREE);
    }

    const fn get(&self, sc: usize, i: usize) -> usize {
        let bit_offset = (1usize << (Self::LOG_UNITS - sc)) + i;
        let shift = bit_offset & (Self::BITS_IN_WORD - 1);
        let entry = self.table[bit_offset >> Self::LOG_BITS_IN_WORD];
        ((entry >> shift) & MASK) as usize
    }

    const fn set(&mut self, sc: usize, i: usize, v: usize) {
        let bit_offset = (1usize << (Self::LOG_UNITS - sc)) + i;
        let shift = bit_offset & (Self::BITS_IN_WORD - 1);
        let entry = self.table[bit_offset >> Self::LOG_BITS_IN_WORD];
        let new_entry = ((v as u64) << shift) | (entry & !(MASK << shift));
        self.table[bit_offset >> Self::LOG_BITS_IN_WORD] = new_entry;
    }

    const fn get_unit(&self, size_class: usize, unit: usize) -> usize {
        self.get(size_class, Self::index_in_size_class(size_class, unit))
    }

    const fn set_unit(&mut self, size_class: usize, unit: usize, v: usize) {
        self.set(size_class, Self::index_in_size_class(size_class, unit), v)
    }

    const fn size_class(units: usize) -> usize {
        units.next_power_of_two().trailing_zeros() as usize
    }

    const fn entries_in_size_class(size_class: usize) -> usize {
        1usize << (Self::LOG_UNITS - size_class)
    }

    const fn words_in_size_class(size_class: usize) -> usize {
        1usize << (Self::LOG_UNITS - Self::LOG_BITS_IN_WORD - size_class)
    }

    const fn index_in_size_class(size_class: usize, unit: usize) -> usize {
        unit >> size_class
    }

    const fn sibling_unit(size_class: usize, unit: usize) -> usize {
        unit ^ (1 << size_class)
    }

    const fn parent_unit(size_class: usize, unit: usize) -> usize {
        unit & !((1 << (size_class + 1)) - 1)
    }

    fn search_and_allocate_cell(&mut self, size_class: usize) -> Option<usize> {
        if Self::LOG_UNITS - Self::LOG_BITS_IN_WORD >= size_class {
            let base = 1usize << (Self::LOG_UNITS - Self::LOG_BITS_IN_WORD - size_class);
            for i in 0..Self::words_in_size_class(size_class) {
                if self.table[base + i] != 0 {
                    for j in 0..Self::BITS_IN_WORD {
                        let index = (i << Self::LOG_BITS_IN_WORD) + j;
                        if self.get(size_class, index) == FREE {
                            self.set(size_class, index, USED);
                            return Some(index << size_class);
                        }
                    }
                }
            }
        } else {
            for i in 0..Self::entries_in_size_class(size_class) {
                if self.get(size_class, i) == FREE {
                    self.set(size_class, i, USED);
                    return Some(i << size_class);
                }
            }
        }
        None
    }

    /// Allocate a power-of-two cell
    fn acquire_cell(&mut self, size_class: usize) -> Option<usize> {
        if size_class > Self::LOG_UNITS {
            return None;
        }
        if let Some(cell) = self.search_and_allocate_cell(size_class) {
            return Some(cell);
        }
        // Split from parent cell
        let parent_size_class = size_class + 1;
        let parent_unit = self.acquire_cell(parent_size_class)?;
        self.set_unit(parent_size_class, parent_unit, USED);
        let child_units = (parent_unit, parent_unit + (1 << size_class));
        self.set_unit(size_class, child_units.0, USED);
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
            if self.get_unit(size_class, sibling) == FREE {
                self.set_unit(size_class, unit, USED);
                self.set_unit(size_class, sibling, USED);
                let parent = Self::parent_unit(size_class, unit);
                let parent_size_class = size_class + 1;
                debug_assert_eq!(self.get_unit(parent_size_class, parent), USED);
                self.release_cell(parent_size_class, parent)
            }
        }
    }

    fn acquire_units(&mut self, units: usize) -> Option<usize> {
        let size_class = Self::size_class(units);
        let start = self.acquire_cell(size_class)?;
        if units != (1 << size_class) {
            let free_start = start + units;
            let free_units = (1 << size_class) - units;
            self.release_units(free_start, free_units);
        }
        Some(start)
    }

    fn release_units(&mut self, mut start: usize, mut units: usize) {
        let limit = start + units;
        while start < limit {
            let curr_size_class = Self::size_class(units);
            let prev_size_class = if units == (1 << curr_size_class) {
                curr_size_class
            } else {
                curr_size_class - 1
            };
            let size_class = usize::min(prev_size_class, start.trailing_zeros() as usize);
            let size = 1usize << size_class;
            let end = start + size;
            debug_assert_eq!(start & (size - 1), 0);
            debug_assert!(end <= limit);
            self.release_cell(size_class, start);
            start = end;
            units = limit - end;
        }
    }

    pub fn acquire<S: PageSize>(&mut self, pages: usize) -> Range<Page<S>> {
        let small_pages = pages << (S::LOG_BYTES - Size4K::LOG_BYTES);
        let unit = self.acquire_units(small_pages).unwrap();
        let addr = self.base + (unit << Size4K::LOG_BYTES);
        let page = Page::new(addr);
        page..Page::forward(page, pages)
    }

    pub fn release<S: PageSize>(&mut self, pages: Range<Page<S>>) {
        let start = (pages.start.start() - self.base) >> Size4K::LOG_BYTES;
        let units = Page::steps_between(&pages.start, &pages.end).unwrap()
            << (S::LOG_BYTES - Size4K::LOG_BYTES);
        self.release_units(start, units);
    }
}
