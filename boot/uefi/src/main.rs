#![no_std]
#![no_main]
#![feature(global_asm)]
#![feature(llvm_asm)]
#![feature(alloc_error_handler)]
#![feature(format_args_nl)]
#![feature(core_intrinsics)]
#![feature(box_syntax)]
#![feature(never_type)]
#![feature(step_trait_ext)]
#![feature(const_fn_transmute)]
#![feature(const_in_array_repeat_expressions)]
#![feature(untagged_unions)]

use core::{alloc::{GlobalAlloc, Layout}, intrinsics::transmute, panic::PanicInfo, ptr};
use cortex_a::regs::*;
use proton_kernel::page_table::*;
use proton::memory::*;
use proton_kernel::page_table::PageFlags;
use uefi::{
    prelude::*,
    table::boot::*,
};
#[macro_use]
mod log;
use elf_rs::*;

pub struct NoAlloc;

unsafe impl GlobalAlloc for NoAlloc {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        unreachable!()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        unreachable!()
    }
}

#[global_allocator]
static ALLOCATOR: NoAlloc = NoAlloc;

static mut BOOT_SYSTEM_TABLE: Option<SystemTable<Boot>> = None;

fn boot_system_table() -> &'static SystemTable<Boot> {
    unsafe { BOOT_SYSTEM_TABLE.as_ref().unwrap() }
}

#[cfg(debug_assertions)]
static KERNEL_ELF: &'static [u8] = include_bytes!("../../../target/aarch64-unknown-none/debug/proton");

#[cfg(not(debug_assertions))]
static KERNEL_ELF: &'static [u8] = include_bytes!("../../../target/aarch64-unknown-none/release/proton");

fn new_page4k() -> Frame {
    let page = boot_system_table().boot_services().allocate_pages(AllocateType::AnyPages, MemoryType::LOADER_DATA, 1).unwrap().unwrap();
    let page = Frame::new(Address::from(page as usize));
    unsafe { page.zero() };
    page
}

fn get_next_table<L: TableLevel>(table: &mut PageTable<L>, index: usize) -> Option<&'static mut PageTable<L::NextLevel>> {
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
    table.entries[index].set(frame, PageFlags::kernel_code_flags_2m() | PageFlag::SMALL_PAGE);
    log!("Mapped {:?} -> {:?}", page, frame);
}

fn map_kernel_pages_4k(p4: &mut PageTable<L4>, start: u64, pages: usize) {
    for i in 0..pages {
        map_kernel_page_4k(p4, Page::new(Address::from((start + ((i as u64) << 12)) as usize)));
    }
}

fn invalidate_tlb() {
    unsafe {
        llvm_asm! {"
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
        + TCR_EL1::EPD1::EnableTTBR1Walks
    );
    TCR_EL1.set(TCR_EL1.get() | 0b101 << 32); // Intermediate Physical Address Size (IPS) = 0b101
    TCR_EL1.set(TCR_EL1.get() | 0x10 <<  0);  // TTBR0_EL1 memory size (T0SZ) = 0x10 ==> 2^(64 - T0SZ)
    TCR_EL1.set(TCR_EL1.get() | 0x10 << 16);  // TTBR1_EL1 memory size (T1SZ) = 0x10 ==> 2^(64 - T1SZ)
    invalidate_tlb();
    log!("Setup TCR Done");
}

fn load_elf() -> extern "C" fn(isize, *const *const u8) -> isize {
    log!("Parse Kernel ELF");
    let elf = Elf::from_bytes(KERNEL_ELF).unwrap();
    log!("Parse Kernel ELF Done");
    if let Elf::Elf64(elf) = elf {
        let entry: extern fn(isize, *const *const u8) = unsafe { ::core::mem::transmute(elf.header().entry_point()) };
        log!("Entry @ {:?}", entry as *mut ());
        let mut load_start = None;
        let mut load_end = None;
        for p in elf.program_header_iter().filter(|p| p.ph.ph_type() == ProgramType::LOAD) {
            // log!("{:?}", p.ph);
            let start: Address = (p.ph.vaddr() as usize).into();
            let end = start + (p.ph.filesz() as usize);
            match (load_start, load_end) {
                (None, None) => {
                    load_start = Some(start);
                    load_end = Some(end);
                }
                (Some(s), Some(e)) => {
                    if start < s { load_start = Some(start) }
                    if end   > e { load_end = Some(end) }
                }
                _ => unreachable!()
            }
        }
        let vaddr_start = Page::<Size4K>::align(load_start.unwrap());
        let vaddr_end = Page::<Size4K>::align_up(load_end.unwrap());
        let pages = ((vaddr_end - vaddr_start) + ((1 << 12) - 1)) >> 12;
        log!("Map code start");
        map_kernel_pages_4k(unsafe { &mut *(TTBR0_EL1.get() as *mut PageTable<L4>) }, vaddr_start.as_usize() as _, pages);
        log!("Map code end");
        // Copy data
        log!("Copy code start");
        for p in elf.program_header_iter().filter(|p| p.ph.ph_type() == ProgramType::LOAD) {
            let start: Address = (p.ph.vaddr() as usize).into();
            let bytes = p.ph.filesz() as usize;
            let offset = p.ph.offset() as usize;
            let src = &KERNEL_ELF[offset] as *const u8;
            let dst = start.as_ptr_mut::<u8>();
            unsafe { ptr::copy_nonoverlapping(src, dst, bytes); }
        }
        log!("Copy code end");

        unsafe { transmute(entry) }
    } else {
        unimplemented!("elf32 is not supported")
    }
}

#[no_mangle]
pub extern "C" fn efi_main(_image: Handle, st: SystemTable<Boot>) -> Status {
    unsafe {
        BOOT_SYSTEM_TABLE = Some(st.unsafe_clone());
    }

    log!("Hello, UEFI!");

    unsafe { setup_tcr(); }

    let start = load_elf();

    log!("Starting kernel...");

    let ret = start(0, 0 as _);

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
