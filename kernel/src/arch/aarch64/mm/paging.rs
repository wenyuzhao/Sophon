use core::iter::Step;
use cortex_a::regs::*;
use cortex_a::barrier;
use super::page_table::*;
use crate::memory::*;
use crate::heap::constants::*;
use super::super::constants::*;
use super::super::uart::boot_time_log;
use super::page_table::PageFlags;

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
        let ptr = GPIO_BASE & !0xFFFF0000_00000000;
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

pub unsafe fn setup_kernel_pagetables() {// Query VC memory
    // Get video-core occupied memory
    boot_time_log("[boot: setup_kernel_pagetables 0]");
    // <0x3c000000 2M> <0x40000000 2M>
    let (vcm_start, vcm_end) = {
        // use crate::mailbox::*;
        // let res::GetVCMemory { base_address, size } = match MailBox::boottime_send(Channel::PropertyARM2VC, req::GetVCMemory) {
        //     Ok(x) => x,
        //     Err(e) => {
        //         crate::debug_boot::log("[boot: setup_kernel_pagetables -> boottime_send failed]");
        //         panic!()
        //     }
        // };
        // boot_log!("x");
        // let base_address = 0xFE00_0000usize;
        // let size = 0x100_0000usize;
        // let start = Address::<P>::new(base_address as _);
        // let end = start + size as usize;
        let start = Address::<P>::new(PERIPHERAL_BASE & !0xFFFF0000_00000000);
        let end = start + 0x1000000 as usize;
        (Frame::<Size2M>::new(start), Frame::<Size2M>::new(end))
    };
    boot_time_log("[boot: setup_kernel_pagetables 1]");
    // Reserve frames for later identity mapping
    
    boot_time_log("[boot: setup_kernel_pagetables 2]");
    // Set some extra MMU attributes
    // const T0SZ: u64 = 0x10 << 0;
    // const T1SZ: u64 = 0x10 << 16;
    // TCR_EL1.set(T0SZ | T1SZ);

    // if !ID_AA64MMFR0_EL1.matches_all(ID_AA64MMFR0_EL1::TGran64::Supported) {
    //     crate::debug_boot::log("Error: TGran64 Not Supported");
    //     loop {}
    // }

    // #[allow(non_upper_case_globals)]
    // const MT_DEVICE_nGnRnE: usize = 0;
    // #[allow(non_upper_case_globals)]
    // const MT_DEVICE_nGnRE: usize = 1;
    // const MT_DEVICE_GRE:   usize = 2;
    // const MT_NORMAL_NC:    usize = 3;
    // const MT_NORMAL:       usize = 4;
    // MAIR_EL1.set(
    //     (0x00u64 << (MT_DEVICE_nGnRnE * 8)) |
    //     (0x04u64 << (MT_DEVICE_nGnRE * 8)) |
    //     (0x0cu64 << (MT_DEVICE_GRE * 8)) |
    //     (0x44u64 << (MT_NORMAL_NC * 8)) |
    //     (0xffu64 << (MT_NORMAL * 8))
    // );
    MAIR_EL1.write(
        // Attribute 1 - Cacheable normal DRAM.
        MAIR_EL1::Attr1_HIGH::Memory_OuterWriteBack_NonTransient_ReadAlloc_WriteAlloc
         + MAIR_EL1::Attr1_LOW_MEMORY::InnerWriteBack_NonTransient_ReadAlloc_WriteAlloc
        // Attribute 0 - Device.
         + MAIR_EL1::Attr0_HIGH::Device
         + MAIR_EL1::Attr0_LOW_DEVICE::Device_nGnRE,
    );

    
    boot_time_log("[boot: setup_kernel_pagetables 3]");
    setup_ttbr0_el1();
    setup_ttbr1_el1();

    boot_time_log("[boot: setup_kernel_pagetables 3.1]");
    
    assert!(TCR_EL1.get() == 0);
    // let ips = ID_AA64MMFR0_EL1.read(ID_AA64MMFR0_EL1::PARange);
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

    TCR_EL1.set(TCR_EL1.get());

    boot_time_log("[boot: setup_kernel_pagetables 4]");
    // Enable MMU
    barrier::isb(barrier::SY);
    // Enable the MMU and turn on data and instruction caching.
    SCTLR_EL1.modify(SCTLR_EL1::M::Enable + SCTLR_EL1::C::Cacheable + SCTLR_EL1::I::Cacheable);
    // Force MMU init to complete before next instruction.
    barrier::isb(barrier::SY);

    // SCTLR_EL1.set(SCTLR_EL1.get() | 0x1);
    // boot_log!("xxx");
    // loop {}
    boot_time_log("[boot: setup_kernel_pagetables 5]");
    {
        // Stack + Kernel: 0x0 ~ KERNEL_HEAP_END
            boot_time_log("mau 1");
        let blocks = (kernel_heap_end() & 0x0000ffff_ffffffff) >> Size2M::LOG_SIZE;
        boot_time_log("mau 2");
        mark_as_used::<Size2M>(Frame::new(0x0.into()), blocks);
        boot_time_log("mau 3");
        for f in vcm_start..=vcm_end {
            boot_time_log("mau 4");
            mark_as_used::<Size2M>(f, 1);
        }
        boot_time_log("mau 5");
    }
    boot_time_log("[boot: setup_kernel_pagetables 5.1]");
    // Map kernel code
    let kernel_code_start = KERNEL_START & 0x0000ffff_ffffffff;
    boot_time_log("[boot: setup_kernel_pagetables 5.1]");
    let kernel_code_end = kernel_end() * 0x0000ffff_ffffffff;
    boot_time_log("[boot: setup_kernel_pagetables 5.1]");
    let kernel_code_start_frame = Frame::<Size4K>::new(kernel_code_start.into());
    boot_time_log("[boot: setup_kernel_pagetables 5.1]");
    if kernel_code_end >= kernel_code_start {
        boot_time_log(">=");
    } else {
        boot_time_log("<");
        if kernel_code_end == 0 {
            boot_time_log("herlen_end == 0");
        }
    }
    let a = kernel_code_end - kernel_code_start;
    boot_time_log("[boot: setup_kernel_pagetables 5.1==]");
    let frames = (a + Size4K::MASK) >> Size4K::LOG_SIZE;
    boot_time_log("[boot: setup_kernel_pagetables 5.1]");
    identity_map_kernel_memory_nomark::<Size4K>(kernel_code_start_frame, frames, PageFlags::_KERNEL_STACK_FLAGS);
    boot_time_log("[boot: setup_kernel_pagetables 6]");
    // Map core 0 kernel stack
    let start_start = KERNEL_CORE0_STACK_START & 0x0000ffff_ffffffff;
    let pages = (KERNEL_CORE0_STACK_END - KERNEL_CORE0_STACK_START) >> Size4K::LOG_SIZE;
    identity_map_kernel_memory_nomark::<Size4K>(Frame::new(start_start.into()), pages, PageFlags::_KERNEL_STACK_FLAGS);
    boot_time_log("[boot: setup_kernel_pagetables 7]");
    // Map kernel heap
    let kernel_heap_start = kernel_heap_start() & 0x0000ffff_ffffffff;
    let kernel_heap_start_frame = Frame::<Size4K>::new(kernel_heap_start.into());
    // boot_log!("{:?} {:?}", kernel_heap_start_frame, KERNEL_HEAP_PAGES);
    identity_map_kernel_memory_nomark::<Size4K>(kernel_heap_start_frame, KERNEL_HEAP_PAGES, PageFlags::_KERNEL_DATA_FLAGS_4K);
    boot_time_log("[boot: setup_kernel_pagetables 8]");
    
    // Map VC Memory
    let p4 = PageTable::<L4>::get(true);
    for f in vcm_start..vcm_end {
        p4.identity_map::<Size2M>(f, PageFlags::_DEVICE_MEMORY_FLAGS_2M);
    }
    // println!("xxx {:?} {:?}", vcm_start, vcm_end);
    boot_time_log("[boot: setup_kernel_pagetables 9]");
    // Mark ARM Generic Timer Mapped Memory
    let arm_frame = Frame::<Size2M>::new(ARM_TIMER_BASE.into());
    boot_time_log("[boot: setup_kernel_pagetables 10]");
    p4.identity_map::<Size2M>(arm_frame, PageFlags::_DEVICE_MEMORY_FLAGS_2M);
    // boot_log!("arm_frame {:?} {:?}", arm_frame, arm_frame.start() + Size2M::SIZE);
    boot_time_log("[boot: setup_kernel_pagetables 11]");
    // boot_log!("xxx {:?} {:?}", vcm_start, vcm_end);
    // crate::debug_boot::log("[boot: setup_kernel_pagetables 10]");
}

fn mark_as_used<S: PageSize>(start_frame: Frame<S>, n_frames: usize) {
    boot_time_log("mark_as_used 2");
    let limit_frame = start_frame.add_usize(n_frames).unwrap();
    // Mark frames as used
    boot_time_log("mark_as_used");
    let mut frame = start_frame;
    boot_time_log("mark_as_used");
    while frame < limit_frame {
        boot_time_log("mark_as_used");

    // for frame in start_frame..limit_frame {
        boot_time_log("mark_as_used a");
        super::frame_allocator::mark_as_used(frame);
        boot_time_log("mark_as_used b");

        frame = frame.add_one();
    }
}

fn identity_map_kernel_memory_nomark<S: PageSize>(start_frame: Frame<S>, n_frames: usize, flags: PageFlags) {
    // mark_as_used::<Size2M>(f, 1);
    let limit_frame = start_frame.add_usize(n_frames).unwrap();
    let p4 = PageTable::<L4>::get(true);
    for frame in start_frame..limit_frame {
        // crate::debug_boot::log("[boot: translate start]");
        if p4.translate(Address::<V>::new(frame.start().as_usize())).is_none() {
            // crate::debug_boot::log("[boot: identity_map start]");
            p4.identity_map(frame, flags);
            // crate::debug_boot::log("[boot: identity_map end]");
        } else {
            // crate::debug_boot::log("[boot: unreachable start]");
            unreachable!()
        }
    }
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