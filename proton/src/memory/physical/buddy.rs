use super::PhysicalPageResource;
use crate::utils::{address::*, page::*};
use core::ops::Range;

const LOG_MAX_ADDRESS_SPACE_SIZE: usize = 48;
const NUM_SIZE_CLASS: usize = LOG_MAX_ADDRESS_SPACE_SIZE - Size4K::LOG_BYTES + 1;

pub struct Buddy {
    table: [Address<P>; NUM_SIZE_CLASS],
}

impl Buddy {
    pub const fn new() -> Self {
        Self {
            table: [Address::ZERO; NUM_SIZE_CLASS],
        }
    }

    const fn size(size_class: usize) -> usize {
        1 << (size_class + Size4K::LOG_BYTES)
    }

    const fn size_class(size: usize) -> usize {
        size.next_power_of_two().trailing_zeros() as usize - Size4K::LOG_BYTES
    }

    #[inline(always)]
    fn push(&mut self, cell: Address<P>, size_class: usize) {
        unsafe {
            cell.store(self.table[size_class]);
        }
        self.table[size_class] = cell;
    }

    #[inline(always)]
    fn pop(&mut self, size_class: usize) -> Option<Address<P>> {
        if self.table[size_class].is_zero() {
            return None;
        }
        let cell = self.table[size_class];
        unsafe {
            self.table[size_class] = cell.load();
        }
        Some(cell)
    }

    fn split_cell(
        &mut self,
        parent: Address<P>,
        parent_size_class: usize,
    ) -> (Address<P>, Address<P>) {
        let child_size_class = parent_size_class - 1;
        let unit1 = parent;
        let unit2 = parent + (1 << (child_size_class + Size4K::LOG_BYTES));
        (unit1, unit2)
    }

    #[cold]
    fn allocate_cell_slow(&mut self, request_size_class: usize) -> Option<Address<P>> {
        for size_class in request_size_class..NUM_SIZE_CLASS {
            if let Some(unit) = self.pop(size_class) {
                let parent = unit;
                for parent_size_class in ((request_size_class + 1)..=size_class).rev() {
                    let (unit1, unit2) = self.split_cell(parent, parent_size_class);
                    let child_size_class = parent_size_class - 1;
                    // Add second cell to list
                    debug_assert!(child_size_class < NUM_SIZE_CLASS);
                    self.push(unit2, child_size_class);
                }
                return Some(unit);
            }
        }
        None
    }

    #[inline(always)]
    fn allocate_cell(&mut self, size_class: usize) -> Option<Address<P>> {
        if let Some(cell) = self.pop(size_class) {
            return Some(cell);
        }
        self.allocate_cell_slow(size_class)
    }

    #[inline(always)]
    fn release_cell(&mut self, cell: Address<P>, size_class: usize) {
        self.push(cell, size_class);
    }

    fn release_contiguous(&mut self, mut start: Address<P>, mut size: usize) {
        let limit = start + size;
        while start < limit {
            let curr_size_class = Self::size_class(size);
            let prev_size_class = if size == Self::size(curr_size_class) {
                curr_size_class
            } else {
                curr_size_class - 1
            };
            let size_class = usize::min(
                prev_size_class,
                start.trailing_zeros() as usize - Size4K::LOG_BYTES,
            );
            let end = start + Self::size(size_class);
            if (*start & (Self::size(size_class) - 1)) != 0 {
                loop {}
            }
            debug_assert_eq!((*start & (Self::size(size_class) - 1)), 0);
            if *end > *limit {
                loop {}
            }
            debug_assert!(*end <= *limit);
            self.release_cell(start, size_class);
            start = end;
            size = limit - end;
        }
        debug_assert_eq!(start, limit);
    }
}

impl PhysicalPageResource for Buddy {
    fn init(&mut self, frames: &'static [Range<Frame>]) {
        for range in frames {
            let start = range.start.start();
            let end = range.end.start();
            self.release_contiguous(start, end - start);
        }
    }

    #[inline(always)]
    fn acquire<S: PageSize>(&mut self) -> Option<Frame<S>> {
        let size = 1 << S::LOG_BYTES;
        let size_class = Self::size_class(size);
        let addr = self.allocate_cell(size_class)?;
        Some(Frame::new(addr))
    }

    #[inline(always)]
    fn release<S: PageSize>(&mut self, frame: Frame<S>) {
        let size = 1 << S::LOG_BYTES;
        let size_class = Self::size_class(size);
        self.release_cell(frame.start(), size_class);
    }
}
