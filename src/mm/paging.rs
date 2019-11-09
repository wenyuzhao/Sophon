use core::iter::Step;
use cortex_a::regs::*;
use super::address::*;
use super::page::*;
use super::page_table::*;
use super::heap_constants::*;

#[repr(C, align(4096))]
struct TempFrames([usize; 512], [usize; 512]);

static mut TEMP_FRAMES: TempFrames = TempFrames([0; 512], [0; 512]);
static mut KERNEL_P4_LOW: PageTable<L4> = PageTable::new();
static mut KERNEL_P4_HIGH: PageTable<L4> = PageTable::new();

const fn get_index(a: usize, level: usize) -> usize {
    let shift = (level - 1) * 9 + 12;
    (a >> shift) & 0b111111111
}

unsafe fn setup_ttbr0_el1() {
    // Identity map 0x0 ~ 0x200000
    let p3 = &TEMP_FRAMES.0 as *const _ as usize as *mut PageTable<L3>;
    let p2 = &TEMP_FRAMES.1 as *const _ as usize as *mut PageTable<L2>;
    let flags = PageFlags::SMALL_PAGE | PageFlags::PRESENT | PageFlags::ACCESSED | PageFlags::OUTER_SHARE;
    // Map p3 to p4
    KERNEL_P4_LOW.entries[get_index(p3 as _, 4)].set(Frame::<Size4K>::new((p3 as usize).into()), flags);
    // Map p2 tp p3
    (*p3).entries[get_index(p2 as _, 3)].set(Frame::<Size4K>::new((p2 as usize).into()), flags);
    // Map first block to p2
    (*p2).entries[0].set(Frame::<Size2M>::new(0usize.into()), PageFlags::PRESENT | PageFlags::ACCESSED | PageFlags::OUTER_SHARE);
    // Set page table register 0
    KERNEL_P4_LOW.entries[511].set::<Size4K>(Frame::new(Address::from(&KERNEL_P4_LOW as *const _)), PageFlags::SMALL_PAGE | PageFlags::OUTER_SHARE | PageFlags::ACCESSED | PageFlags::PRESENT);
    let p4 = &KERNEL_P4_LOW as *const PageTable<L4>;
    TTBR0_EL1.set(p4 as u64 & 0x0000ffff_ffffffff);
}

unsafe fn setup_ttbr1_el1() {
    // Setup TTTBR1_EL1 recursive mapping
    KERNEL_P4_HIGH.entries[511].set::<Size4K>(Frame::new(Address::from(&KERNEL_P4_HIGH as *const _)), PageFlags::SMALL_PAGE | PageFlags::OUTER_SHARE | PageFlags::ACCESSED | PageFlags::PRESENT);
    // Set page table
    let p4 = &KERNEL_P4_HIGH as *const PageTable<L4>;
    TTBR1_EL1.set(p4 as u64 & 0x0000ffff_ffffffff);
}

pub fn clear_temp_user_pagetable() {
    unsafe {
        for i in 0..511 {
            KERNEL_P4_LOW.entries[i].clear();
        }
    }
}

pub unsafe fn setup_kernel_pagetables() {// Query VC memory
    // Get video-core occupied memory
    let (vcm_start, vcm_end) = {
        use crate::mailbox::*;
        let res::GetVCMemory { base_address, size } = MailBox::boottime_send(Channel::PropertyARM2VC, req::GetVCMemory).unwrap();
        let start = Address::<P>::new(base_address as _);
        let end = start + size as usize;
        (Frame::<Size2M>::new(start), Frame::<Size2M>::new(end))
    };
    // Reserve frames for later identity mapping
    {
        // Stack + Kernel: 0x0 ~ KERNEL_HEAP_END
        let blocks = (kernel_heap_end() & 0x0000ffff_ffffffff) >> Size2M::LOG_SIZE;
        mark_as_used::<Size2M>(Frame::new(0x0.into()), blocks);
        for f in vcm_start..vcm_end {
            mark_as_used::<Size2M>(f, 1);
        }
    }
    setup_ttbr0_el1();
    setup_ttbr1_el1();
    // Set some extra MMU attributes
    const T0SZ: u64 = 0x10 << 0;
    const T1SZ: u64 = 0x10 << 16;
    TCR_EL1.set(T0SZ | T1SZ);
    #[allow(non_upper_case_globals)]
    const MT_DEVICE_nGnRnE: usize = 0;
    #[allow(non_upper_case_globals)]
    const MT_DEVICE_nGnRE: usize = 1;
    const MT_DEVICE_GRE:   usize = 2;
    const MT_NORMAL_NC:    usize = 3;
    const MT_NORMAL:       usize = 4;
    MAIR_EL1.set(
        (0x00u64 << (MT_DEVICE_nGnRnE * 8)) |
        (0x04u64 << (MT_DEVICE_nGnRE * 8)) |
        (0x0cu64 << (MT_DEVICE_GRE * 8)) |
        (0x44u64 << (MT_NORMAL_NC * 8)) |
        (0xffu64 << (MT_NORMAL * 8))
    );
    // Enable MMU
    SCTLR_EL1.set(SCTLR_EL1.get() | 0x1);
    // Map core 0 kernel stack
    let start_start = KERNEL_CORE0_STACK_START & 0x0000ffff_ffffffff;
    let pages = (KERNEL_CORE0_STACK_END - KERNEL_CORE0_STACK_START) >> Size4K::LOG_SIZE;
    identity_map_kernel_memory_nomark::<Size4K>(Frame::new(start_start.into()), pages, true);
    // Map core 0 kernel code + heap
    let kernel_start = KERNEL_START & 0x0000ffff_ffffffff;
    // First 2M block
    identity_map_kernel_memory_nomark::<Size4K>(Frame::new(kernel_start.into()), (0x200000 - kernel_start) >> Size4K::LOG_SIZE, true);
    // Remaining blocks
    let kernel_end = kernel_heap_end() & 0x0000ffff_ffffffff;
    let blocks = ((kernel_end - 0x200000) + ((1 << Size2M::LOG_SIZE) - 1)) / (1 << Size2M::LOG_SIZE);
    identity_map_kernel_memory_nomark::<Size2M>(Frame::new(0x200000.into()), blocks, true);
    // Map VC Memory
    let p4 = PageTable::<L4>::get(true);
    for f in vcm_start..vcm_end {
        p4.identity_map::<Size2M>(f, PageFlags::OUTER_SHARE | PageFlags::ACCESSED | PageFlags::PRESENT);
    }
    // Mark ARM Generic Timer Mapped Memory
    let arm_frame = Frame::<Size4K>::new(crate::timer::ARM_TIMER_BASE.into());
    p4.identity_map::<Size4K>(arm_frame, PageFlags::SMALL_PAGE | PageFlags::OUTER_SHARE | PageFlags::ACCESSED | PageFlags::PRESENT);
}

fn mark_as_used<S: PageSize>(start_frame: Frame<S>, n_frames: usize) {
    let limit_frame = start_frame.add_usize(n_frames).unwrap();
    // Mark frames as used
    for frame in start_frame..limit_frame {
        super::frame_allocator::mark_as_used(frame);
    }
}

fn identity_map_kernel_memory_nomark<S: PageSize>(start_frame: Frame<S>, n_frames: usize, high_address: bool) {
    let limit_frame = start_frame.add_usize(n_frames).unwrap();
    // Setup page table
    let p4 = PageTable::<L4>::get(high_address);
    let mut flags = PageFlags::OUTER_SHARE | PageFlags::ACCESSED;
    if S::LOG_SIZE == Size4K::LOG_SIZE {
        flags |= PageFlags::SMALL_PAGE;
    }
    for frame in start_frame..limit_frame {
        if p4.translate(Address::<V>::new(frame.start().as_usize())).is_none() {
            p4.identity_map(frame, flags);
        } else {
            unreachable!()
        }
    }
}

pub fn fork_page_table(parent_p4_frame: Frame, stack_frame: Frame<Size2M>) -> Frame {
    PageTable::<L4>::with_temporary_low_table(parent_p4_frame, |parent_p4| {
        let frame = parent_p4.fork(stack_frame, false);
        

        {
            let page = crate::mm::map_kernel_temporarily(frame, PAGE_TABLE_FLAGS);
            let new_table = unsafe { page.start().as_ref_mut::<PageTable<L4>>() };
            new_table.entries[511].set(frame, super::page_table::PAGE_TABLE_FLAGS);
        }

        frame
        // parent_p4.mark_as_copy_on_write();
        // let child_p4_frame = super::frame_allocator::alloc::<Size4K>().unwrap();
        // let child_p4_page = crate::mm::map_kernel_temporarily(child_p4_frame, PageFlags::OUTER_SHARE | PageFlags::ACCESSED);
        // let child_p4 = unsafe { child_p4_page.start().as_ref_mut::<PageTable<L4>>() };
        // for i in 0..parent_p4.entries.len() {
        //     child_p4.entries[i] = parent_p4.entries[i].clone();
        // }
        // child_p4_frame
    })
}