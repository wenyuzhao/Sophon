use core::iter::Step;
use cortex_a::regs::*;
use super::address::*;
use super::page::*;
use super::page_table::*;
use super::heap_constants::*;

#[repr(C, align(4096))]
struct PageTableTTBR0([usize; 512], [usize; 512], [usize; 512], [usize; 512]);

static mut PT0: PageTableTTBR0 = PageTableTTBR0([0; 512], [0; 512], [0; 512], [0; 512]);

const fn get_index(a: usize, level: usize) -> usize {
    let shift = (level - 1) * 9 + 12;
    (a >> shift) & 0b111111111
}

unsafe fn setup_ttbr0_el1() {
    // Identity map 0x0 ~ 0x200000
    let p4 = &PT0.0 as *const _ as usize as *mut [usize; 512];
    assert!(p4 as usize & 0xffff_0000_0000_0000 == 0);
    let p3 = &PT0.1 as *const _ as usize as *mut [usize; 512];
    let p2 = &PT0.2 as *const _ as usize as *mut [usize; 512];
    // Map p3 to p4
    (*p4)[get_index(p3 as _, 4)] = (p3 as usize | 0x3 | (1 << 10));
    // Map p2 tp p3
    (*p3)[get_index(p2 as _, 3)] = (p2 as usize | 0x3 | (1 << 10));
    // Map first block to p2
    (*p2)[0] = (0usize | 0x1 | (1 << 10));
    (*p2)[505] = (0x3F201000 | 0x1 | (1 << 10));
    // Set page table register 0
    TTBR0_EL1.set(p4 as _);
}

static mut KERNEL_P4: PageTable<L4> = PageTable::new();

pub unsafe fn setup_kernel_pagetables() {
    setup_ttbr0_el1();
    // Map KERNEL_P4 to itself
    KERNEL_P4.entries[511].set::<Size4K>(Frame::new(Address::from(&KERNEL_P4 as *const _)), PageFlags::SMALL_PAGE | PageFlags::OUTER_SHARE | PageFlags::ACCESSED | PageFlags::PRESENT);
    // Set page table
    let p4 = &KERNEL_P4 as *const PageTable<L4>;
    TTBR1_EL1.set(p4 as u64 & 0x0000ffff_ffffffff);
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
    // Reserve frames for later identity mapping
    {
        // Stack + Kernel: 0x0 ~ KERNEL_HEAP_END
        let blocks = (kernel_heap_end() & 0x0000ffff_ffffffff) >> Size2M::LOG_SIZE;
        mark_as_used::<Size2M>(Frame::new(0x0.into()), blocks);
        // MMIO: 0x3F000000 ~ 0x40000000
        let mmio_start = Frame::<Size2M>::new((crate::gpio::PERIPHERAL_BASE & 0x0000ffff_ffffffff).into());
        mark_as_used::<Size2M>(mmio_start, 8);
    }
    // Map core 0 kernel stack
    let start_start = KERNEL_CORE0_STACK_START & 0x0000ffff_ffffffff;
    let pages = (KERNEL_CORE0_STACK_END - KERNEL_CORE0_STACK_START) >> Size4K::LOG_SIZE;
    identity_map_kernel_memory_nomark::<Size4K>(Frame::new(start_start.into()), pages);
    // Map core 0 kernel code + heap
    let kernel_start = KERNEL_START & 0x0000ffff_ffffffff;
    // First 2M block
    identity_map_kernel_memory_nomark::<Size4K>(Frame::new(kernel_start.into()), (0x200000 - kernel_start) >> Size4K::LOG_SIZE);
    // Remaining blocks
    let kernel_end = kernel_heap_end() & 0x0000ffff_ffffffff;
    let blocks = ((kernel_end - 0x200000) + ((1 << Size2M::LOG_SIZE) - 1)) / (1 << Size2M::LOG_SIZE);
    identity_map_kernel_memory_nomark::<Size2M>(Frame::new(0x200000.into()), blocks);
    // Map MMIO
    let mmio_start = Frame::<Size2M>::new((crate::gpio::PERIPHERAL_BASE & 0x0000ffff_ffffffff).into());
    identity_map_kernel_memory_nomark::<Size2M>(mmio_start, 8);
}

fn mark_as_used<S: PageSize>(start_frame: Frame<S>, n_frames: usize) {
    let limit_frame = start_frame.add_usize(n_frames).unwrap();
    // Mark frames as used
    for frame in start_frame..limit_frame {
        super::frame_allocator::mark_as_used(frame);
    }
}

fn identity_map_kernel_memory_nomark<S: PageSize>(start_frame: Frame<S>, n_frames: usize) {
    let limit_frame = start_frame.add_usize(n_frames).unwrap();
    // Setup page table
    let p4 = PageTable::<L4>::get();
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

pub fn identity_map_kernel_memory<S: PageSize>(fb: Address<P>, size: usize) -> Address<V> {
    let start_frame = Frame::<S>::of(fb);
    let mut end_frame = Frame::<S>::of(fb + size);
    if Frame::<S>::is_aligned(fb + size) {
        end_frame = end_frame.sub_one();
    }
    // Setup frame allocator
    for frame in start_frame..=end_frame {
        super::frame_allocator::mark_as_used(frame);
    }
    // Setup page table
    let p4 = PageTable::<L4>::get();
    let mut flags = PageFlags::OUTER_SHARE | PageFlags::ACCESSED;
    if S::LOG_SIZE == Size4K::LOG_SIZE {
        flags |= PageFlags::SMALL_PAGE;
    }
    for frame in start_frame..=end_frame {
        if p4.translate(Address::<V>::new(frame.start().as_usize())).is_none() {
            p4.identity_map(frame, flags);
        }
    }
    let fb_high = fb.as_usize() | 0xffff_0000_0000_0000;
    fb_high.into() 
}