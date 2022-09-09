use core::{iter::Step, ops::Range};

use crate::{memory::kernel::KERNEL_MEMORY_MAPPER, task::MMState};

use super::kernel::KERNEL_MEMORY_RANGE;
use super::physical::PHYSICAL_MEMORY;
use alloc::sync::Arc;
use atomic::Ordering;
use memory::{
    page::{Frame, Page, PageSize, Size1G, Size2M, Size4K},
    page_table::*,
};
use proc::Proc;

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

pub fn sbrk(proc: Arc<dyn Proc>, num_pages: usize) -> Option<Range<Page<Size4K>>> {
    let result = MMState::of(&*proc).virtual_memory_highwater.fetch_update(
        Ordering::SeqCst,
        Ordering::SeqCst,
        |old| {
            let old_aligned = old.align_up(Size4K::BYTES);
            Some(old_aligned + (num_pages << Size4K::LOG_BYTES))
        },
    );
    // log!("sbrk: {:?} {:?}", self.id, result);
    match result {
        Ok(a) => {
            let old_top = a;
            let start = Page::new(a.align_up(Size4K::BYTES));
            let end = Page::forward(start, num_pages);
            debug_assert_eq!(old_top, start.start());
            // Map old_top .. end
            {
                let page_table = MMState::of(&*proc).get_page_table();
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
