use core::{
    iter::Step,
    ops::Range,
    sync::atomic::{AtomicBool, AtomicPtr},
};

use crate::memory::kernel::KERNEL_MEMORY_MAPPER;

use super::kernel::KERNEL_MEMORY_RANGE;
use super::physical::PHYSICAL_MEMORY;
use alloc::{boxed::Box, sync::Arc};
use atomic::{Atomic, Ordering};
use klib::proc::{MemSpace, Process};
use memory::{
    address::{Address, V},
    page::{Frame, Page, PageSize, Size1G, Size2M, Size4K},
    page_table::*,
};

pub fn fork_mem_space(mem: &MemSpace) -> Box<MemSpace> {
    if !mem.has_user_page_table() {
        warn!("fork_mem_space: no user page table");
        // The process has no user space.
        return Box::new(MemSpace {
            page_table: AtomicPtr::new(mem.page_table.load(Ordering::SeqCst)),
            has_user_page_table: AtomicBool::new(false),
            highwater: Atomic::new(mem.highwater.load(Ordering::SeqCst)),
        });
    }
    // Traverse the page table, set every entry to copy-on-write
    let cloned_p4 = {
        let _guard = KERNEL_MEMORY_MAPPER.with_kernel_address_space();
        let page_table = mem.get_page_table();
        fn mark_cow(entry: &mut PageTableEntry, _vaddr: Address<V>, _level: usize) {
            if !entry.flags().contains(PageFlags::USER) {
                return;
            }
            let mut flags = entry.flags();
            flags |= PageFlags::COPY_ON_WRITE | PageFlags::NO_WRITE;
            entry.update_flags(flags);
        }
        page_table.walk_mut(&mut mark_cow);
        let page_table_cloned = page_table.clone(&PHYSICAL_MEMORY);
        page_table_cloned.walk_mut(&mut mark_cow);
        page_table_cloned
    };
    let mem_space = MemSpace {
        page_table: AtomicPtr::new(cloned_p4),
        has_user_page_table: AtomicBool::new(true),
        highwater: Atomic::new(mem.highwater.load(Ordering::SeqCst)),
    };
    Box::new(mem_space)
}

pub fn release_user_page_table<L: TableLevel>(page_table: &mut PageTable<L>) {
    for i in 0..512 {
        if L::ID == L4::ID && i >= PageTable::<L4>::get_index(KERNEL_MEMORY_RANGE.start) {
            break;
        }
        if page_table[i].is_empty() || !page_table[i].present() {
            continue;
        }
        if page_table[i].is_block() || L::ID == L1::ID {
            let page = page_table[i].address();
            match L::ID {
                L1::ID => PHYSICAL_MEMORY.release::<Size4K>(Frame::new(page)),
                L2::ID => PHYSICAL_MEMORY.release::<Size2M>(Frame::new(page)),
                L3::ID => PHYSICAL_MEMORY.release::<Size1G>(Frame::new(page)),
                _ => unreachable!(),
            }
        } else {
            let next_page_table = page_table[i].address();
            release_user_page_table::<L::NextLevel>(unsafe { next_page_table.as_mut() });
        }
    }
    PHYSICAL_MEMORY.release::<Size4K>(Frame::new(page_table.into()));
}

pub fn sbrk(proc: Arc<Process>, num_pages: usize) -> Option<Range<Page<Size4K>>> {
    let result = proc
        .mem
        .highwater
        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |old| {
            let old_aligned = old.align_up(Size4K::BYTES);
            Some(old_aligned + (num_pages << Size4K::LOG_BYTES))
        });
    // log!("sbrk: {:?} {:?}", self.id, result);
    match result {
        Ok(a) => {
            let old_top = a;
            let start = Page::new(a.align_up(Size4K::BYTES));
            let end = Page::forward(start, num_pages);
            debug_assert_eq!(old_top, start.start());
            // Map old_top .. end
            {
                let page_table = proc.mem.get_page_table();
                let _guard = KERNEL_MEMORY_MAPPER.with_kernel_address_space();
                for page in start..end {
                    let frame = PHYSICAL_MEMORY.acquire().unwrap();
                    page_table.map(
                        page,
                        frame,
                        PageFlags::user_data_flags_4k(),
                        &PHYSICAL_MEMORY,
                    );
                }
            }
            Some(start..end)
        }
        Err(_e) => return None,
    }
}
