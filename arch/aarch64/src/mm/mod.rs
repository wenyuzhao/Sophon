pub mod frame_allocator;
pub mod page_table;
pub mod paging;
use page_table::PageFlags as ArchPageFlags;
use page_table::{PageTable, L4};
use proton::memory::*;
use proton_kernel::arch::*;
use proton_kernel::task::*;
use crate::Kernel;
use crate::arch::*;

pub struct MemoryManager;

impl MemoryManager {
}

impl AbstractMemoryManager for MemoryManager {
    fn alloc_frame<S: PageSize>() -> Frame<S> {
        frame_allocator::alloc().unwrap()
    }
    fn dealloc_frame<S: PageSize>(frame: Frame<S>) {
        frame_allocator::free(frame)
    }
    fn map<S: PageSize>(page: Page<S>, frame: Frame<S>, flags: PageFlags) {
        let p4 = PageTable::<L4>::get(page.start().as_usize() & 0xffff_0000_0000_0000 != 0);
        p4.map(page, frame, to_arch_flags::<S>(flags));
    }
    fn translate(address: Address<V>) -> Option<(Address<P>, PageFlags)> {
        let p4 = PageTable::<L4>::get(address.as_usize() & 0xffff_0000_0000_0000 != 0);
        p4.translate(address).map(|(a, f)| (a, to_flags(f)))
    }
    fn update_flags<S: PageSize>(page: Page<S>, flags: PageFlags) {
        let p4 = PageTable::<L4>::get(page.start().as_usize() & 0xffff_0000_0000_0000 != 0);
        p4.update_flags(page, to_arch_flags::<S>(flags));
    }
    fn unmap<S: PageSize>(page: Page<S>) {
        let p4 = PageTable::<L4>::get(page.start().as_usize() & 0xffff_0000_0000_0000 != 0);
        p4.unmap(page);
    }
    fn map_user<S: PageSize>(task: TaskId, page: Page<S>, frame: Frame<S>, flags: PageFlags) {
        <AArch64 as AbstractArch>::Interrupt::uninterruptable(|| {
            let ctx = &Task::<Kernel>::by_id(task).unwrap().context;
            // Set pagetable
            unsafe {
                llvm_asm! {"
                    msr	ttbr0_el1, $0
                    tlbi vmalle1is
                    DSB ISH
                    isb
                "
                ::   "r"(ctx.p4.start().as_usize())
                }
            }
            let p4 = PageTable::<L4>::get(false);
            p4.map(page, frame, to_arch_flags::<S>(flags));
        })
    }
}

fn to_arch_flags<S: PageSize>(flags: PageFlags) -> ArchPageFlags {
    let mut aflags = ArchPageFlags::empty();
    // const PRESENT     = 1 << 0;
    // const ACCESSED    = 1 << 1;
    // const KERNEL      = 1 << 2;
    // const NO_WRITE    = 1 << 3;
    // const NO_EXEC     = 1 << 4;
    if flags.contains(PageFlags::PRESENT) {
        aflags |= ArchPageFlags::PRESENT;
    }
    if flags.contains(PageFlags::ACCESSED) {
        aflags |= ArchPageFlags::ACCESSED;
    }
    if !flags.contains(PageFlags::KERNEL) {
        aflags |= ArchPageFlags::USER;
    }
    if flags.contains(PageFlags::NO_WRITE) {
        aflags |= ArchPageFlags::NO_WRITE;
    }
    if flags.contains(PageFlags::NO_EXEC) {
        aflags |= ArchPageFlags::NO_EXEC;
    }
    if S::SIZE == Size4K::SIZE {
        aflags |= ArchPageFlags::SMALL_PAGE;
    }
    aflags |= ArchPageFlags::OUTER_SHARE;
    aflags |= ArchPageFlags::NORMAL_MEMORY;
    aflags
}


fn to_flags(aflags: ArchPageFlags) -> PageFlags {
    let mut flags = PageFlags::empty();
    // const PRESENT     = 1 << 0;
    // const ACCESSED    = 1 << 1;
    // const KERNEL      = 1 << 2;
    // const NO_WRITE    = 1 << 3;
    // const NO_EXEC     = 1 << 4;
    if aflags.contains(ArchPageFlags::PRESENT) {
        flags |= PageFlags::PRESENT;
    }
    if aflags.contains(ArchPageFlags::ACCESSED) {
        flags |= PageFlags::ACCESSED;
    }
    if !aflags.contains(ArchPageFlags::USER) {
        flags |= PageFlags::KERNEL;
    }
    if aflags.contains(ArchPageFlags::NO_WRITE) {
        flags |= PageFlags::NO_WRITE;
    }
    if aflags.contains(ArchPageFlags::NO_EXEC) {
        flags |= PageFlags::NO_EXEC;
    }
    flags
}

pub fn handle_user_pagefault(address: Address) {
    let p4 = PageTable::<L4>::get(false);
    if let Some((_, flags)) = p4.translate(address) {
        if flags.contains(ArchPageFlags::COPY_ON_WRITE) {
            p4.fix_copy_on_write(address, !flags.contains(ArchPageFlags::SMALL_PAGE));
            return
        }
    }
    debug!(crate::Kernel: "Page Fault at {:?}", address);
    unimplemented!()
}

pub fn is_copy_on_write_address(address: Address) -> bool {
    let p4 = PageTable::<L4>::get(false);
    if let Some((_, flags)) = p4.translate(address) {
        flags.contains(ArchPageFlags::COPY_ON_WRITE)
    } else {
        false
    }
}

pub fn fix_copy_on_write_address(address: Address) {
    debug_assert!(is_copy_on_write_address(address));
    let p4 = PageTable::<L4>::get(false);
    let (_, flags) = p4.translate(address).unwrap();
    p4.fix_copy_on_write(address, !flags.contains(ArchPageFlags::SMALL_PAGE));
}
