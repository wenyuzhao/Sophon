#![no_std]
#![no_main]
#![feature(asm)]
#![feature(format_args_nl)]
#![feature(core_intrinsics)]
#![feature(step_trait)]

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;
use core::iter::Step;
use core::{intrinsics::transmute, mem, ops::Range, ptr, slice};
use cortex_a::registers::*;
use elf_rs::*;
use proton::memory::page_table::*;
use proton::utils::address::*;
use proton::utils::page::*;
use proton::BootInfo;
use tock_registers::interfaces::{Readable, Writeable};
use uefi::proto::loaded_image::LoadedImage;
use uefi::proto::media::file::*;
use uefi::{prelude::*, table::boot::*};

#[macro_use]
mod log;

static mut BOOT_SYSTEM_TABLE: Option<SystemTable<Boot>> = None;

fn boot_system_table() -> &'static SystemTable<Boot> {
    unsafe { BOOT_SYSTEM_TABLE.as_ref().unwrap() }
}

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

fn map_kernel_page_4k(p4: &mut PageTable<L4>, page: Page<Size4K>) {
    fn get_next_table<L: TableLevel>(
        p: &mut PageTable<L>,
        i: usize,
    ) -> &'static mut PageTable<L::NextLevel> {
        if p[i].present() && !p[i].is_block() {
            let addr = p[i].address();
            unsafe { transmute(addr) }
        } else {
            panic!()
        }
    }
    let table = p4;
    // Get p3
    let index = PageTable::<L4>::get_index(page.start());
    if table[index].is_empty() {
        table[index].set(new_page4k(), PageFlags::page_table_flags());
    }
    let table = get_next_table(table, index);
    // Get p2
    let index = PageTable::<L3>::get_index(page.start());
    if table[index].is_empty() {
        table[index].set(new_page4k(), PageFlags::page_table_flags());
    }
    let table = get_next_table(table, index);
    // Get p1
    let index = PageTable::<L2>::get_index(page.start());
    if table[index].is_empty() {
        table[index].set(new_page4k(), PageFlags::page_table_flags());
    }
    let table = get_next_table(table, index);
    // Map
    let index = PageTable::<L1>::get_index(page.start());
    let frame = new_page4k();
    table[index].set(frame, PageFlags::kernel_code_flags_4k());
    // log!("Mapped {:?} -> {:?}", page, frame);
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

fn load_elf(elf_data: &[u8]) -> extern "C" fn(&mut BootInfo) -> isize {
    log!("Parse Kernel ELF");
    let elf = Elf::from_bytes(elf_data).unwrap();
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
        let vaddr_end = load_end.unwrap().align_up(Size4K::BYTES);
        let pages = ((vaddr_end - vaddr_start) + ((1 << 12) - 1)) >> 12;
        log!("Map code start");
        map_kernel_pages_4k(PageTable::<L4>::get(), vaddr_start.as_usize() as _, pages);
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
            let src = &elf_data[offset] as *const u8;
            let dst = start.as_mut_ptr::<u8>();
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
        .memory_map(unsafe { buffer.start().as_mut::<[u8; 4096]>() })
        .unwrap()
        .unwrap();
    let count = Frame::<Size4K>::BYTES / mem::size_of::<Range<Frame>>();
    let available_physical_memory_ranges: &'static mut [Range<Frame>] =
        unsafe { slice::from_raw_parts_mut(buffer.start().as_mut_ptr(), count) };
    let mut cursor = 0;
    for desc in descriptors {
        if desc.ty == MemoryType::CONVENTIONAL {
            let start = Frame::<Size4K>::new((desc.phys_start as usize).into());
            let end = Step::forward(start, desc.page_count as usize);
            available_physical_memory_ranges[cursor] = start..end;
            cursor += 1;
        }
    }
    let available_physical_memory_ranges = &available_physical_memory_ranges[..cursor];
    log!(
        "Available physical memory: {:?}",
        available_physical_memory_ranges
    );
    return available_physical_memory_ranges;
}

fn gen_boot_info(device_tree: &'static [u8]) -> BootInfo {
    BootInfo {
        available_physical_memory: gen_available_physical_memory(),
        device_tree,
    }
}

fn read_file(handle: Handle, path: &str) -> Vec<u8> {
    let sfs = unsafe {
        &mut *boot_system_table()
            .boot_services()
            .get_image_file_system(handle)
            .unwrap()
            .expect("Cannot open `SimpleFileSystem` protocol")
            .get()
    };
    let mut directory = sfs.open_volume().unwrap().unwrap();
    let file = directory
        .open(path, FileMode::Read, FileAttribute::empty())
        .unwrap()
        .unwrap()
        .into_type()
        .unwrap()
        .unwrap();
    if let FileType::Regular(mut file) = file {
        let mut buffer = vec![];
        let mut buf = vec![0; 4096];
        let mut total_size = 0usize;
        loop {
            let size = file.read(&mut buf).unwrap().unwrap();
            if size == 0 {
                break;
            } else {
                total_size += size;
                buffer.extend_from_slice(&buf);
            }
        }
        buffer.resize(total_size, 0);
        buffer
    } else {
        panic!("{:?} is not a file.", path);
    }
}

fn read_dtb(handle: Handle) -> Vec<u8> {
    let loaded_image = unsafe {
        &*boot_system_table()
            .boot_services()
            .handle_protocol::<LoadedImage>(handle)
            .unwrap()
            .expect("Failed to retrieve `LoadedImage` protocol from handle")
            .get()
    };
    let mut buf = vec![0; 4096];
    let mut args = loaded_image.load_options(&mut buf).unwrap().split(" ");
    let dtb_path = args
        .find(|x| x.starts_with("dtb="))
        .map(|x| x.strip_prefix("dtb=").unwrap())
        .expect("Device tree not specified");
    log!("Load device tree: {}", dtb_path);
    read_file(handle, dtb_path)
}

#[no_mangle]
pub extern "C" fn efi_main(image: Handle, st: SystemTable<Boot>) -> Status {
    uefi_services::init(&st).expect_success("Failed to initialize utilities");
    unsafe {
        BOOT_SYSTEM_TABLE = Some(st.unsafe_clone());
    }

    log!("Hello, UEFI!");

    unsafe {
        setup_tcr();
    }

    log!("Loading kernel...");

    let kernel_elf = read_file(image, "proton");
    let dtb = read_dtb(image);
    let start = load_elf(&kernel_elf);

    log!("Starting kernel...");

    let mut boot_info = gen_boot_info(dtb.leak());
    let buffer = new_page4k();
    let buffer = unsafe { buffer.start().as_mut::<[u8; 4096]>() };
    st.boot_services().memory_map(buffer).unwrap().unwrap();
    st.exit_boot_services(image, buffer).unwrap_success();

    let ret = start(&mut boot_info);

    log!("Kernel return {}", ret);

    loop {}
}

#[no_mangle]
pub extern "C" fn __chkstk() {}
