use spin::Mutex;
use crate::mm::address::*;
use crate::mm::page::*;
use crate::mm::heap_constants::*;
use core::ops::Index;

const SMALL_FRAMES_IN_HEAP: usize = MAX_HEAP_SIZE >> Size4K::LOG_SIZE;
const LARGE_FRAMES_IN_HEAP: usize = MAX_HEAP_SIZE >> Size2M::LOG_SIZE;

type UIntEntry = u128;
const LOG_BITS_IN_ENTRY: usize = 7;
const BITS_IN_ENTRY: usize = 1 << 7;
const BITMAP_ENTRIES_4K: usize = SMALL_FRAMES_IN_HEAP / BITS_IN_ENTRY;
const BITMAP_ENTRIES_2M: usize = LARGE_FRAMES_IN_HEAP / BITS_IN_ENTRY;

const LOG_PAGES_IN_BLOCK: usize = Size2M::LOG_SIZE - Size4K::LOG_SIZE;
const PAGES_IN_BLOCK: usize = 1 << LOG_PAGES_IN_BLOCK;

struct BitMapAllocator {
    map4k: [UIntEntry; BITMAP_ENTRIES_4K],
    map2m: [UIntEntry; BITMAP_ENTRIES_2M],
}

impl BitMapAllocator {
    const fn new() -> Self {
        Self {
            map4k: [UIntEntry::max_value(); BITMAP_ENTRIES_4K],
            map2m: [0; BITMAP_ENTRIES_2M],
        }
    }
    fn get_4k(&self, i: usize) -> bool {
        let entry_index = i >> LOG_BITS_IN_ENTRY;
        let bit_index = i & ((1 << LOG_BITS_IN_ENTRY) - 1);
        self.map4k[entry_index] & (1 << bit_index) != 0
    }
    fn set_4k(&mut self, i: usize, v: bool) {
        let entry_index = i >> LOG_BITS_IN_ENTRY;
        let bit_index = i & ((1 << LOG_BITS_IN_ENTRY) - 1);
        if v {
            debug_assert!(!self.get_4k(i));
            self.map4k[entry_index] |= (1 << bit_index);
        } else {
            debug_assert!(self.get_4k(i));
            self.map4k[entry_index] &= !(1 << bit_index);
        }
    }
    fn get_2m(&self, i: usize) -> bool {
        let entry_index = i >> LOG_BITS_IN_ENTRY;
        let bit_index = i & ((1 << LOG_BITS_IN_ENTRY) - 1);
        let x = self.map2m[entry_index] & (1 << bit_index) != 0;
        x
    }
    fn set_2m(&mut self, i: usize, v: bool) {
        let entry_index = i >> LOG_BITS_IN_ENTRY;
        let bit_index = i & ((1 << LOG_BITS_IN_ENTRY) - 1);
        if v {
            debug_assert!(!self.get_2m(i));
            self.map2m[entry_index] |= (1 << bit_index);
        } else {
            debug_assert!(self.get_2m(i));
            self.map2m[entry_index] &= !(1 << bit_index);
        }
    }
    fn alloc4k(&mut self) -> Option<usize> {
        // Find a empty 4k slot
        for i in 0..self.map4k.len() {
            let entry = self.map4k[i];
            if entry != UIntEntry::max_value() {
                for j in 0..BITS_IN_ENTRY {
                    if entry & (1 << j) == 0 {
                        let index = i * BITS_IN_ENTRY + j;
                        debug_assert!(self.get_2m(index >> LOG_PAGES_IN_BLOCK));
                        self.set_4k(index, true);
                        return Some(index);
                    }
                }
            }
        }
        // Find a empty 2m slot
        for i in 0..self.map2m.len() {
            let entry = self.map2m[i];
            if entry != UIntEntry::max_value() {
                for j in 0..BITS_IN_ENTRY {
                    if entry & (1 << j) == 0 {
                        let index_2m = i * BITS_IN_ENTRY + j;
                        self.set_2m(index_2m, true);
                        let index_4k = index_2m << LOG_PAGES_IN_BLOCK;
                        for k in 1..PAGES_IN_BLOCK {
                            self.set_4k(index_4k + k, false);
                        }
                        return Some(index_4k);
                    }
                }
            }
        }
        None
    }
    fn alloc2m(&mut self) -> Option<usize> {
        // Find a empty 2m slot
        for i in 0..self.map2m.len() {
            let entry = self.map2m[i];
            if entry != UIntEntry::max_value() {
                for j in 0..BITS_IN_ENTRY {
                    if entry & (1 << j) == 0 {
                        let index = i * BITS_IN_ENTRY + j;
                        self.set_2m(index, true);
                        return Some(index);
                    }
                }
            }
        }
        None
    }
    
    fn free4k(&mut self, index: usize) {
        let entry_index = index >> LOG_BITS_IN_ENTRY;
        let bit_index = index & ((1 << LOG_BITS_IN_ENTRY) - 1);
        self.map4k[entry_index] &= !(1 << bit_index);
        let all_freed = {
            let mut all_freed = true;
            let start = (index >> LOG_PAGES_IN_BLOCK) << LOG_PAGES_IN_BLOCK;
            for k in 0..PAGES_IN_BLOCK {
                if self.get_4k(start + k) {
                    all_freed = false;
                    break;
                }
            }
            all_freed
        };
        if all_freed {
            let index_2m = index >> LOG_PAGES_IN_BLOCK;
            let entry_index = index_2m >> LOG_BITS_IN_ENTRY;
            let bit_index = index_2m & ((1 << LOG_BITS_IN_ENTRY) - 1);
            self.map2m[entry_index] &= !(1 << bit_index);
            let index_4k = index_2m << LOG_PAGES_IN_BLOCK;
            for k in 0..PAGES_IN_BLOCK {
                self.set_4k(index_4k + k, true);
            }
        }
    }
    fn free2m(&mut self, index: usize) {
        let entry_index = index >> LOG_BITS_IN_ENTRY;
        let bit_index = index & ((1 << LOG_BITS_IN_ENTRY) - 1);
        self.map2m[entry_index] &= !(1 << bit_index);
        let index_4k = index << LOG_PAGES_IN_BLOCK;
        for k in 0..PAGES_IN_BLOCK {
            self.set_4k(index_4k + k, true);
        }
    }
}

static ALLOCATOR: Mutex<BitMapAllocator> = Mutex::new(BitMapAllocator::new());

pub fn mark_as_used<S: PageSize>(frame: Frame<S>) {
    let mut allocator = ALLOCATOR.lock();
    if S::LOG_SIZE == Size4K::LOG_SIZE {
        let index_2m = frame.start().as_usize() >> Size2M::LOG_SIZE;
        if !allocator.get_2m(index_2m) {
            allocator.set_2m(index_2m, true);
            let index_4k = index_2m << LOG_PAGES_IN_BLOCK;
            for i in 0..PAGES_IN_BLOCK {
                allocator.set_4k(index_4k + i, false);
            }
        }
        let index_4k = frame.start().as_usize() >> Size4K::LOG_SIZE;
        allocator.set_4k(index_4k, true);
    } else {
        let index_2m = frame.start().as_usize() >> Size2M::LOG_SIZE;
        allocator.set_2m(index_2m, true);
    }
}

use core::sync::atomic::{Ordering, AtomicBool};
static AB: AtomicBool = AtomicBool::new(false);

pub fn alloc<S: PageSize>() -> Option<Frame<S>> {
    let mut allocator = ALLOCATOR.lock();
    if S::LOG_SIZE == Size4K::LOG_SIZE {
        let addr = Address::<P>::new(allocator.alloc4k()? << Size4K::LOG_SIZE);
        Some(Frame::new(addr))
    } else {
        let addr = Address::<P>::new(allocator.alloc2m()? << Size2M::LOG_SIZE);
        Some(Frame::new(addr))
    }
}

pub fn free<S: PageSize>(frame: Frame<S>) {
    let mut allocator = ALLOCATOR.lock();
    if S::LOG_SIZE == Size4K::LOG_SIZE {
        unimplemented!()
    } else {
        allocator.free2m(frame.start().as_usize() >> Size2M::LOG_SIZE);
    }
}
