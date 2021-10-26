use super::kernel::KERNEL_MEMORY_RANGE;
use super::physical::PHYSICAL_MEMORY;
use memory::{
    page::{Frame, Size1G, Size2M, Size4K},
    page_table::*,
};

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
