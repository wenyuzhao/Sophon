use super::{KERNEL_HEAP_RANGE, KERNEL_HEAP_SIZE};
use crate::utils::unint_lock::UnintMutex;
use core::{iter::Step, ops::Range};
use memory::{address::Address, page::*};

pub static VIRTUAL_PAGE_ALLOCATOR: UnintMutex<VirtualPageAllocator<KERNEL_HEAP_SIZE>> =
    UnintMutex::new(VirtualPageAllocator::new(KERNEL_HEAP_RANGE.start));

pub struct VirtualPageAllocator<const HEAP_SIZE: usize>
where
    [(); HEAP_SIZE / Size4K::BYTES / 64]: Sized,
    [(); HEAP_SIZE / Size2M::BYTES / 64]: Sized,
    [(); HEAP_SIZE / Size1G::BYTES / 64]: Sized,
{
    base: Address,
    table_4k: [u64; HEAP_SIZE / Size4K::BYTES / 64],
    table_2m: [u64; HEAP_SIZE / Size2M::BYTES / 64],
    table_1g: [u64; HEAP_SIZE / Size1G::BYTES / 64],
}

impl<const HEAP_SIZE: usize> VirtualPageAllocator<HEAP_SIZE>
where
    [(); HEAP_SIZE / Size4K::BYTES / 64]: Sized,
    [(); HEAP_SIZE / Size2M::BYTES / 64]: Sized,
    [(); HEAP_SIZE / Size1G::BYTES / 64]: Sized,
{
    const LOG_BITS_IN_ENTRY: usize = 6;
    const BITS_IN_ENTRY: usize = 1 << Self::LOG_BITS_IN_ENTRY;

    pub const fn new(base: Address) -> Self {
        Self {
            base,
            table_4k: [0u64; HEAP_SIZE / Size4K::BYTES / 64],
            table_2m: [0u64; HEAP_SIZE / Size2M::BYTES / 64],
            table_1g: [0u64; HEAP_SIZE / Size1G::BYTES / 64],
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

    pub fn acquire<S: PageSize>(&mut self, pages: usize) -> Range<Page<S>> {
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

        let page = Page::<S>::new(self.base + (start_index << S::LOG_BYTES));
        page..Page::forward(page, pages)
    }

    pub fn release<S: PageSize>(&self, _pages: Range<Page<S>>) {}
}
