#![no_std]
#![no_main]
#![feature(global_asm)]
#![feature(asm)]
#![feature(alloc_error_handler)]
#![feature(format_args_nl)]
#![feature(core_intrinsics)]
#![feature(box_syntax)]
#![feature(never_type)]
#![feature(step_trait_ext)]
#![feature(const_fn_transmute)]
#![feature(untagged_unions)]
#![feature(step_trait)]

extern crate alloc;

use core::alloc::Layout;
use core::iter::Step;
use core::{intrinsics::transmute, mem, ops::Range, panic::PanicInfo, ptr, slice};
use cortex_a::regs::*;
use proton::utils::address::*;
use proton::utils::page::*;
use proton::{page_table::PageFlags, utils::no_alloc::NoAlloc};
use proton::{page_table::*, BootInfo};
use uefi::{prelude::*, table::boot::*};
#[macro_use]
mod log;
use elf_rs::*;

static DEVICE_TREE: &'static [u8] = include_bytes!("../../dtbs/qemu-virt.dtb");

#[global_allocator]
static ALLOCATOR: NoAlloc = NoAlloc;

static mut BOOT_SYSTEM_TABLE: Option<SystemTable<Boot>> = None;

fn boot_system_table() -> &'static SystemTable<Boot> {
    unsafe { BOOT_SYSTEM_TABLE.as_ref().unwrap() }
}

#[cfg(debug_assertions)]
static KERNEL_ELF: &'static [u8] =
    include_bytes!("../../../target/aarch64-unknown-none/debug/proton");

#[cfg(not(debug_assertions))]
static KERNEL_ELF: &'static [u8] =
    include_bytes!("../../../target/aarch64-unknown-none/release/proton");

fn new_page4k() -> Frame {
    let page = boot_system_table()
        .boot_services()
        .allocate_pages(AllocateType::AnyPages, MemoryType::LOADER_DATA, 1)
        .unwrap()
        .unwrap();
    let page = Frame::new(Address::from(page as usize));
    unsafe { page.zero() };
    page
}

fn get_next_table<L: TableLevel>(
    table: &mut PageTable<L>,
    index: usize,
) -> Option<&'static mut PageTable<L::NextLevel>> {
    if table.entries[index].present() && !table.entries[index].is_block() {
        let addr = table.entries[index].address();
        Some(unsafe { transmute(addr) })
    } else {
        None
    }
}

fn map_kernel_page_4k(p4: &mut PageTable<L4>, page: Page<Size4K>) {
    let table = p4;
    // Get p3
    let index = PageTable::<L4>::get_index(page.start());
    if table.entries[index].is_empty() {
        table.entries[index].set(new_page4k(), PageFlags::page_table_flags());
    }
    let table = get_next_table(table, index).unwrap();
    // Get p2
    let index = PageTable::<L3>::get_index(page.start());
    if table.entries[index].is_empty() {
        table.entries[index].set(new_page4k(), PageFlags::page_table_flags());
    }
    let table = get_next_table(table, index).unwrap();
    // Get p1
    let index = PageTable::<L2>::get_index(page.start());
    if table.entries[index].is_empty() {
        table.entries[index].set(new_page4k(), PageFlags::page_table_flags());
    }
    let table = get_next_table(table, index).unwrap();
    // Map
    let index = PageTable::<L1>::get_index(page.start());
    let frame = new_page4k();
    table.entries[index].set(
        frame,
        PageFlags::kernel_code_flags_2m() | PageFlag::SMALL_PAGE,
    );
    log!("Mapped {:?} -> {:?}", page, frame);
}

fn map_kernel_pages_4k(p4: &mut PageTable<L4>, start: u64, pages: usize) {
    for i in 0..pages {
        map_kernel_page_4k(
            p4,
            Page::new(Address::from((start + ((i as u64) << 12)) as usize)),
        );
    }
}

fn invalidate_tlb() {
    unsafe {
        asm! {"
            tlbi vmalle1is
            DSB SY
            isb
        "}
    }
}

pub unsafe fn setup_tcr() {
    log!("Setup TCR");
    TCR_EL1.write(
        TCR_EL1::TG0::KiB_4
            + TCR_EL1::TG1::KiB_4
            + TCR_EL1::SH0::Inner
            + TCR_EL1::SH1::Inner
            + TCR_EL1::ORGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable
            + TCR_EL1::IRGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable
            + TCR_EL1::ORGN1::WriteBack_ReadAlloc_WriteAlloc_Cacheable
            + TCR_EL1::IRGN1::WriteBack_ReadAlloc_WriteAlloc_Cacheable
            + TCR_EL1::EPD0::EnableTTBR0Walks
            + TCR_EL1::EPD1::EnableTTBR1Walks,
    );
    TCR_EL1.set(TCR_EL1.get() | 0b101 << 32); // Intermediate Physical Address Size (IPS) = 0b101
    TCR_EL1.set(TCR_EL1.get() | 0x10 << 0); // TTBR0_EL1 memory size (T0SZ) = 0x10 ==> 2^(64 - T0SZ)
    TCR_EL1.set(TCR_EL1.get() | 0x10 << 16); // TTBR1_EL1 memory size (T1SZ) = 0x10 ==> 2^(64 - T1SZ)
    invalidate_tlb();
    log!("Setup TCR Done");
}

fn load_elf() -> extern "C" fn(&mut BootInfo) -> isize {
    log!("Parse Kernel ELF");
    let elf = Elf::from_bytes(KERNEL_ELF).unwrap();
    log!("Parse Kernel ELF Done");
    if let Elf::Elf64(elf) = elf {
        let entry: extern "C" fn(&mut BootInfo) =
            unsafe { ::core::mem::transmute(elf.header().entry_point()) };
        log!("Entry @ {:?}", entry as *mut ());
        let mut load_start = None;
        let mut load_end = None;
        let mut update_load_range = |start: Address, end: Address| match (load_start, load_end) {
            (None, None) => {
                load_start = Some(start);
                load_end = Some(end);
            }
            (Some(s), Some(e)) => {
                if start < s {
                    load_start = Some(start)
                }
                if end > e {
                    load_end = Some(end)
                }
            }
            _ => unreachable!(),
        };
        for p in elf
            .program_header_iter()
            .filter(|p| p.ph.ph_type() == ProgramType::LOAD)
        {
            let start: Address = (p.ph.vaddr() as usize).into();
            let end = start + (p.ph.filesz() as usize);
            update_load_range(start, end);
        }
        if let Some(bss) = elf.lookup_section(".bss") {
            let start = Address::<V>::from(bss.sh.addr() as usize);
            let end = start + bss.sh.size() as usize;
            update_load_range(start, end);
        }
        let vaddr_start = Page::<Size4K>::align(load_start.unwrap());
        let vaddr_end = Page::<Size4K>::align_up(load_end.unwrap());
        let pages = ((vaddr_end - vaddr_start) + ((1 << 12) - 1)) >> 12;
        log!("Map code start");
        let p4 = unsafe { &mut *(TTBR0_EL1.get() as *mut PageTable<L4>) };
        let addr = Address::from(p4 as *mut _);
        p4.entries[511].set(Frame::<Size4K>::new(addr), PageFlags::page_table_flags());
        map_kernel_pages_4k(
            unsafe { &mut *(TTBR0_EL1.get() as *mut PageTable<L4>) },
            vaddr_start.as_usize() as _,
            pages,
        );
        log!("Map code end");
        // Copy data
        log!("Copy code start");
        for p in elf
            .program_header_iter()
            .filter(|p| p.ph.ph_type() == ProgramType::LOAD)
        {
            let start: Address = (p.ph.vaddr() as usize).into();
            let bytes = p.ph.filesz() as usize;
            let offset = p.ph.offset() as usize;
            let src = &KERNEL_ELF[offset] as *const u8;
            let dst = start.as_ptr_mut::<u8>();
            unsafe {
                log!("Copy code {:?}", dst..dst.add(bytes));
                ptr::copy_nonoverlapping(src, dst, bytes);
            }
        }
        log!("Copy code end");

        unsafe { transmute(entry) }
    } else {
        unimplemented!("elf32 is not supported")
    }
}

fn gen_available_physical_memory() -> &'static [Range<Frame>] {
    let bt = boot_system_table();
    let buffer = new_page4k();
    let (_, descriptors) = bt
        .boot_services()
        .memory_map(unsafe { buffer.start().as_ref_mut::<[u8; 4096]>() })
        .unwrap()
        .unwrap();
    let count = Frame::<Size4K>::SIZE / mem::size_of::<Range<Frame>>();
    let available_physical_memory_ranges: &'static mut [Range<Frame>] =
        unsafe { slice::from_raw_parts_mut(buffer.start().as_ptr_mut(), count) };
    let mut cursor = 0;
    for desc in descriptors {
        if desc.ty == MemoryType::CONVENTIONAL {
            let start = Frame::<Size4K>::new((desc.phys_start as usize).into());
            let end = Step::forward(start, desc.page_count as usize);
            available_physical_memory_ranges[cursor] = start..end;
            cursor += 1;
        }
    }
    return available_physical_memory_ranges;
}

fn gen_boot_info() -> BootInfo {
    BootInfo {
        available_physical_memory: gen_available_physical_memory(),
        device_tree: &DEVICE_TREE,
    }
}

#[no_mangle]
pub extern "C" fn efi_main(image: Handle, st: SystemTable<Boot>) -> Status {
    unsafe {
        BOOT_SYSTEM_TABLE = Some(st.unsafe_clone());
    }

    log!("Hello, UEFI!");

    unsafe {
        setup_tcr();
    }

    let start = load_elf();

    let mut boot_info = gen_boot_info();

    log!("Starting kernel...");

    let buffer = new_page4k();
    let buffer = unsafe { buffer.start().as_ref_mut::<[u8; 4096]>() };
    st.boot_services().memory_map(buffer).unwrap().unwrap();
    st.exit_boot_services(image, buffer).unwrap_success();

    let ret = start(&mut boot_info);

    log!("Kernel return {}", ret);

    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    log!("{}", info);
    loop {}
}

#[no_mangle]
pub extern "C" fn __chkstk() {}

#[alloc_error_handler]
fn alloc_error_handler(layout: Layout) -> ! {
    panic!("Allocation error: {:?}", layout)
}
