use core::iter::Step;
use cortex_a::regs::*;
use cortex_a::barrier;
use super::page_table::*;
use proton::memory::*;
use crate::heap::constants::*;
use super::super::uart::boot_time_log;
use super::page_table::PageFlags;
use crate::peripherals::*;
use crate::uart::*;
use core::fmt::Write;

#[repr(C, align(4096))]
struct TempFrames([usize; 512], [usize; 512], [usize; 512], [usize; 512]);

static mut TEMP_FRAMES: TempFrames = TempFrames([0; 512], [0; 512], [0; 512], [0; 512]);
static mut KERNEL_P4_LOW: PageTable<L4> = PageTable::new();
static mut KERNEL_P4_HIGH: PageTable<L4> = PageTable::new();

const fn get_index(a: usize, level: usize) -> usize {
    let shift = (level - 1) * 9 + 12;
    (a >> shift) & 0b111111111
}

unsafe fn setup_ttbr0_el1() {
    // Identity map 0x0 ~ 0x200000
    {
        let ptr = 0x0;
        let p3 = &TEMP_FRAMES.0 as *const _ as usize as *mut PageTable<L3>;
        let p2 = &TEMP_FRAMES.1 as *const _ as usize as *mut PageTable<L2>;
        // Map p3 to p4
        KERNEL_P4_LOW.entries[get_index(ptr as _, 4)].set(Frame::<Size4K>::new((p3 as usize).into()), PageFlags::_PAGE_TABLE_FLAGS);
        // Map p2 tp p3
        (*p3).entries[get_index(ptr as _, 3)].set(Frame::<Size4K>::new((p2 as usize).into()), PageFlags::_PAGE_TABLE_FLAGS);
        // Map first block to p2
        (*p2).entries[get_index(ptr as _, 2)].set(Frame::<Size2M>::new(ptr.into()), PageFlags::_KERNEL_CODE_FLAGS_2M);
    }
    // Identity map 0x3F20_0000 ~ 0x3F21_0000
    // {
    //     let ptr = crate::gpio::PERIPHERAL_BASE & !0xFFFF0000_00000000;
    //     let p3 = &TEMP_FRAMES.0 as *const _ as usize as *mut PageTable<L3>;
    //     let p2 = &TEMP_FRAMES.1 as *const _ as usize as *mut PageTable<L2>;
    //     // Map p3 to p4
    //     KERNEL_P4_LOW.entries[get_index(ptr as _, 4)].set(Frame::<Size4K>::new((p3 as usize).into()), PageFlags::_PAGE_TABLE_FLAGS);
    //     // Map p2 tp p3
    //     (*p3).entries[get_index(ptr as _, 3)].set(Frame::<Size4K>::new((p2 as usize).into()), PageFlags::_PAGE_TABLE_FLAGS);
    //     // Map first block to p2
    //     (*p2).entries[get_index(ptr as _, 2)].set(Frame::<Size2M>::new(ptr.into()), PageFlags::_DEVICE_MEMORY_FLAGS_2M);
    // }
    {
        let ptr = GPIORegisters::BASE_LOW;
        let p3 = &TEMP_FRAMES.0 as *const _ as usize as *mut PageTable<L3>;
        let p2 = &TEMP_FRAMES.1 as *const _ as usize as *mut PageTable<L2>;
        // Map p3 to p4
        KERNEL_P4_LOW.entries[get_index(ptr as _, 4)].set(Frame::<Size4K>::new((p3 as usize).into()), PageFlags::_PAGE_TABLE_FLAGS);
        // Map p2 tp p3
        (*p3).entries[get_index(ptr as _, 3)].set(Frame::<Size4K>::new((p2 as usize).into()), PageFlags::_PAGE_TABLE_FLAGS);
        // Map first block to p2
        (*p2).entries[get_index(ptr as _, 2)].set(Frame::<Size2M>::new(ptr.into()), PageFlags::_DEVICE_MEMORY_FLAGS_2M);
        // let ptr = crate::gpio::GPIO_BASE & !0xFFFF0000_00000000;
        // let p3 = &TEMP_FRAMES.2 as *const _ as usize as *mut PageTable<L3>;
        // let p2 = &TEMP_FRAMES.3 as *const _ as usize as *mut PageTable<L2>;
    // // Map p3 to p4
        // if !KERNEL_P4_LOW.entries[get_index(p3 as _, 4)].present() {
        //     KERNEL_P4_LOW.entries[get_index(p3 as _, 4)].set(Frame::<Size4K>::new((p3 as usize).into()), PageFlags::_PAGE_TABLE_FLAGS);
        // }
    // // Map p2 tp p3
        // if !KERNEL_P4_LOW.entries[get_index(p2 as _, 3)].present() {
        //     KERNEL_P4_LOW.entries[get_index(p2 as _, 3)].set(Frame::<Size4K>::new((p2 as usize).into()), PageFlags::_PAGE_TABLE_FLAGS);
        // }
    // (*p3).entries[get_index(p2 as _, 3)].set(Frame::<Size4K>::new((p2 as usize).into()), PageFlags::_PAGE_TABLE_FLAGS);
    // Map first block to p2
    // (*p2).entries[get_index(ptr as _, 2)].set(Frame::<Size2M>::new(ptr.into()), PageFlags::_KERNEL_CODE_FLAGS_2M);
    }
    
    // Set page table register 0
    KERNEL_P4_LOW.entries[511].set::<Size4K>(Frame::new(Address::from(&KERNEL_P4_LOW as *const _)), PageFlags::_PAGE_TABLE_FLAGS);
    let p4 = &KERNEL_P4_LOW as *const PageTable<L4>;
    TTBR0_EL1.set(p4 as u64 & 0x0000ffff_ffffffff);
}

unsafe fn setup_ttbr1_el1() {
    // Setup TTTBR1_EL1 recursive mapping
    for i in 0..511 {
        KERNEL_P4_HIGH.entries[i].clear();
    }
    KERNEL_P4_HIGH.entries[511].set::<Size4K>(Frame::new(Address::from(&KERNEL_P4_HIGH as *const _)), PageFlags::_PAGE_TABLE_FLAGS);
    // Set page table
    let p4 = &KERNEL_P4_HIGH as *const PageTable<L4>;
    TTBR1_EL1.set(p4 as u64 & 0x0000ffff_ffffffff);
}

pub fn clear_temp_user_pagetable() {
    // unsafe {
    //     for i in 0..511 {
    //         // println!("Clear {}", i);
    //         KERNEL_P4_LOW.entries[i].clear();
    //     }
    // }
    TTBR0_EL1.set(0);
    unsafe {
        asm!("
            tlbi vmalle1is
            DSB SY
            isb
        ")
    }
    // boot_log!("TTBR0_EL1 cleared");
}

pub unsafe fn setup_kernel_pagetables() {
    // Get video-core occupied memory
    boot_time_log("[boot: (mmu) query device memory]");
    let (vcm_start, vcm_end) = {
        // use crate::mailbox::*;
        // let res::GetVCMemory { base_address, size } = match MailBox::boottime_send(Channel::PropertyARM2VC, req::GetVCMemory) {
        //     Ok(x) => x,
        //     Err(e) => {
        //         crate::debug_boot::log("[boot: setup_kernel_pagetables -> boottime_send failed]");
        //         panic!()
        //     }
        // };
        let start = Address::<P>::new(PERIPHERAL_BASE & !0xFFFF0000_00000000);
        let end = start + 0x1000000 as usize;
        (Frame::<Size2M>::new(start), Frame::<Size2M>::new(end))
    };

    boot_time_log("[boot: (mmu) setup MAIR]");
    MAIR_EL1.write(
        // Attribute 1 - Cacheable normal DRAM.
        MAIR_EL1::Attr1_HIGH::Memory_OuterWriteBack_NonTransient_ReadAlloc_WriteAlloc
         + MAIR_EL1::Attr1_LOW_MEMORY::InnerWriteBack_NonTransient_ReadAlloc_WriteAlloc
        // Attribute 0 - Device.
         + MAIR_EL1::Attr0_HIGH::Device
         + MAIR_EL1::Attr0_LOW_DEVICE::Device_nGnRE,
    );

    boot_time_log("[boot: (mmu) setup TTBRx registers]");
    setup_ttbr0_el1();
    setup_ttbr1_el1();

    boot_time_log("[boot: (mmu) setup TCR]");
    assert!(TCR_EL1.get() == 0);
    TCR_EL1.write(
        //   TCR_EL1::IPS.val(0b101)
        TCR_EL1::TG0::KiB_4
        + TCR_EL1::TG1::KiB_4
        + TCR_EL1::SH0::Inner
        + TCR_EL1::SH1::Inner
        + TCR_EL1::ORGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable
        + TCR_EL1::IRGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable
        + TCR_EL1::ORGN1::WriteBack_ReadAlloc_WriteAlloc_Cacheable
        + TCR_EL1::IRGN1::WriteBack_ReadAlloc_WriteAlloc_Cacheable
        + TCR_EL1::EPD0::EnableTTBR0Walks
        + TCR_EL1::EPD1::EnableTTBR1Walks
        // + TCR_EL1::T0SZ.val(0x10)
        // + TCR_EL1::T1SZ.val(0x10)
    );
    TCR_EL1.set(TCR_EL1.get() | 0b101 << 32); // Intermediate Physical Address Size (IPS) = 0b101
    TCR_EL1.set(TCR_EL1.get() | 0x10 <<  0); // TTBR0_EL1 memory size (T0SZ) = 0x10 ==> 2^(64 - T0SZ)
    TCR_EL1.set(TCR_EL1.get() | 0x10 << 16); // TTBR1_EL1 memory size (T1SZ) = 0x10 ==> 2^(64 - T1SZ)


    // Enable MMU and turn on data/instruction caching.
    boot_time_log("[boot: (mmu) enable mmu]");
    barrier::isb(barrier::SY);
    SCTLR_EL1.modify(SCTLR_EL1::M::Enable + SCTLR_EL1::C::Cacheable + SCTLR_EL1::I::Cacheable);
    barrier::isb(barrier::SY);

    // Mark kernel stack/heap and device physical memory as occupied
    boot_time_log("[boot: (mmu) alloc kernel & device frames]");
    let kernel_frames = (kernel_heap_end() & 0x0000ffff_ffffffff) >> Size2M::LOG_SIZE;
    mark_as_used::<Size2M>(Frame::new(0x0.into()), kernel_frames);
    let dev_frames = (vcm_end.start() - vcm_start.start()) >> Size2M::LOG_SIZE;
    mark_as_used::<Size2M>(vcm_start, dev_frames);

    // Map kernel code
    boot_time_log("[boot: (mmu) map kernel code]");
    let kernel_code_start = KERNEL_START & 0x0000ffff_ffffffff;
    let kernel_code_end = kernel_end() & 0x0000ffff_ffffffff;
    let kernel_code_start_frame = Frame::<Size4K>::new(kernel_code_start.into());
    let frames = (kernel_code_end - kernel_code_start + Size4K::MASK) >> Size4K::LOG_SIZE;
    identity_map_kernel_memory_nomark::<Size4K>(kernel_code_start_frame, frames, PageFlags::_KERNEL_STACK_FLAGS);

    // Map core 0 kernel stack
    boot_time_log("[boot: (mmu) map kernel stack]");
    let start_start = KERNEL_CORE0_STACK_START & 0x0000ffff_ffffffff;
    let pages = (KERNEL_CORE0_STACK_END - KERNEL_CORE0_STACK_START) >> Size4K::LOG_SIZE;
    identity_map_kernel_memory_nomark::<Size4K>(Frame::new(start_start.into()), pages, PageFlags::_KERNEL_STACK_FLAGS);

    // Map kernel heap
    boot_time_log("[boot: (mmu) map kernel heap]");
    let kernel_heap_start = kernel_heap_start() & 0x0000ffff_ffffffff;
    let kernel_heap_start_frame = Frame::<Size4K>::new(kernel_heap_start.into());
    identity_map_kernel_memory_nomark::<Size4K>(kernel_heap_start_frame, KERNEL_HEAP_PAGES, PageFlags::_KERNEL_DATA_FLAGS_4K);
    
    // Map device Memory
    boot_time_log("[boot: (mmu) map device memory]");
    let p4 = PageTable::<L4>::get(true);
    for f in vcm_start..vcm_end {
        p4.identity_map::<Size2M>(f, PageFlags::_DEVICE_MEMORY_FLAGS_2M);
    }

    // Mark ARM Generic Timer Mapped Memory
    boot_time_log("[boot: (mmu) map device memory (ARM)]");
    let arm_frame = Frame::<Size2M>::new(ARM_TIMER_BASE.into());
    p4.identity_map::<Size2M>(arm_frame, PageFlags::_DEVICE_MEMORY_FLAGS_2M);
}

fn mark_as_used<S: PageSize>(start_frame: Frame<S>, n_frames: usize) {
    // Mark frames as used
    let limit_frame = start_frame.add_usize(n_frames).unwrap();
    Frame::range(start_frame, limit_frame, |frame| {
        super::frame_allocator::mark_as_used(frame);
    });
}

#[inline(never)]
fn identity_map_kernel_memory_nomark<S: PageSize>(start_frame: Frame<S>, n_frames: usize, flags: PageFlags) {
    let limit_frame = start_frame.add_usize(n_frames).unwrap();
    let p4 = PageTable::<L4>::get(true);
    
    Frame::range(start_frame, limit_frame, |frame| {
        if p4.translate(Address::<V>::new(frame.start().as_usize())).is_none() {
            p4.identity_map(frame, flags);
        } else {
            unreachable!()
        }
    });
}

pub fn fork_page_table(parent_p4_frame: Frame) -> Frame {
    PageTable::<L4>::with_temporary_low_table(parent_p4_frame, |parent_p4| {
        parent_p4.fork()
    })
}

pub fn invalidate_tlb() {
    unsafe {
        asm! {"
            tlbi vmalle1is
            DSB SY
            isb
        "}
    }
}