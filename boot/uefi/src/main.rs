#![no_std]
#![no_main]
#![feature(asm)]
#![feature(format_args_nl)]
#![feature(core_intrinsics)]
#![feature(step_trait)]

extern crate alloc;
#[macro_use]
extern crate log;

use alloc::vec;
use alloc::vec::Vec;
use boot::BootInfo;
use core::iter::Step;
use core::{intrinsics::transmute, mem, ops::Range, ptr, slice};
use cortex_a::registers::*;
use elf_rs::*;
use fdt::Fdt;
use memory::address::*;
use memory::page::*;
use memory::page_table::*;
use tock_registers::interfaces::{ReadWriteable, Readable, Writeable};
use uefi::proto::loaded_image::LoadedImage;
use uefi::proto::media::file::*;
use uefi::Guid;
use uefi::{prelude::*, table::boot::*};

use crate::uefi_logger::UEFILogger;

mod uefi_logger;

static mut BOOT_SYSTEM_TABLE: Option<SystemTable<Boot>> = None;
static mut IMAGE: Option<Handle> = None;

unsafe fn establish_el1_page_table() {
    let p4 = new_page4k().start().as_mut::<PageTable<L4>>();
    TTBR0_EL1.set(p4 as *const _ as u64);
    // Get physical address limit
    let mut buffer = [0u8; 4096];
    let (_, descriptors) = boot_system_table()
        .boot_services()
        .memory_map(&mut buffer)
        .unwrap()
        .unwrap();
    let mut top = Address::<P>::ZERO;
    for desc in descriptors {
        let start = Address::<P>::from(desc.phys_start as *mut u8);
        let end = start + ((desc.page_count as usize) << Size4K::LOG_BYTES);
        if end > top {
            top = end
        }
    }
    // Map pages
    let mut cursor = Address::<V>::ZERO;
    let top = Address::<V>::new(top.as_usize());
    // 1G pages
    while cursor < top {
        identity_map_kernel_page_1g(
            p4,
            if cursor.is_zero() {
                None
            } else {
                Some(Page::new(cursor))
            },
            PageFlags::kernel_code_flags_1g(),
        );
        cursor += Size1G::BYTES;
    }
}

fn boot_system_table() -> &'static SystemTable<Boot> {
    unsafe { BOOT_SYSTEM_TABLE.as_ref().unwrap() }
}

fn new_page4k() -> Frame {
    let page = boot_system_table()
        .boot_services()
        .allocate_pages(AllocateType::AnyPages, MemoryType::LOADER_CODE, 1)
        .unwrap()
        .unwrap();
    let page = Frame::new(Address::from(page as usize));
    unsafe { page.zero() };
    page
}

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

fn translate(p4: &mut PageTable<L4>, page: Page<Size4K>) -> Frame<Size4K> {
    let table = p4;
    // Get p3
    let index = PageTable::<L4>::get_index(page.start());
    let table = get_next_table(table, index);
    // Get p2
    let index = PageTable::<L3>::get_index(page.start());
    let table = get_next_table(table, index);
    // Get p1
    let index = PageTable::<L2>::get_index(page.start());
    let table = get_next_table(table, index);
    // Map
    let index = PageTable::<L1>::get_index(page.start());
    Frame::new(table[index].address())
}

fn identity_map_kernel_page_1g(
    p4: &mut PageTable<L4>,
    page: Option<Page<Size1G>>,
    flags: PageFlags,
) {
    let addr = page.map(|x| x.start()).unwrap_or(Address::ZERO);
    let table = p4;
    // Get p3
    let index = PageTable::<L4>::get_index(addr);
    if !table[index].present() {
        table[index].set(new_page4k(), PageFlags::page_table_flags());
    }
    let table = get_next_table(table, index);
    // Set p3 entry
    let index = PageTable::<L3>::get_index(addr);
    let frame = Frame::<Size1G>::new(Address::new(addr.as_usize()));
    table[index].set(frame, flags);
}

fn map_kernel_page_4k(
    p4: &mut PageTable<L4>,
    page: Page<Size4K>,
    frame: Frame<Size4K>,
    flags: PageFlags,
) {
    fn get_next_table<L: TableLevel>(
        p: &mut PageTable<L>,
        i: usize,
    ) -> &'static mut PageTable<L::NextLevel> {
        if p[i].present() && !p[i].is_block() {
            let addr = p[i].address();
            unsafe { addr.as_mut() }
        } else {
            panic!()
        }
    }
    let table = p4;
    // Get p3
    let index = PageTable::<L4>::get_index(page.start());
    if !table[index].present() {
        table[index].set(new_page4k(), PageFlags::page_table_flags());
    }
    let table = get_next_table(table, index);
    // Get p2
    let index = PageTable::<L3>::get_index(page.start());
    if !table[index].present() {
        table[index].set(new_page4k(), PageFlags::page_table_flags());
    }
    let table = get_next_table(table, index);
    // Get p1
    let index = PageTable::<L2>::get_index(page.start());
    if !table[index].present() {
        table[index].set(new_page4k(), PageFlags::page_table_flags());
    }
    let table = get_next_table(table, index);
    // Map
    let index = PageTable::<L1>::get_index(page.start());
    table[index].set(frame, flags);
}

fn map_kernel_pages_4k(p4: &mut PageTable<L4>, start: u64, pages: usize) {
    for i in 0..pages {
        map_kernel_page_4k(
            p4,
            Page::new(Address::from((start + ((i as u64) << 12)) as usize)),
            new_page4k(),
            PageFlags::kernel_code_flags_4k(),
        );
    }
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
        log!("Map code {:?}", vaddr_start..vaddr_end);
        let p4 = TTBR0_EL1.get() as *mut PageTable<L4>;
        map_kernel_pages_4k(unsafe { &mut *p4 }, vaddr_start.as_usize() as _, pages);
        log!("Map code end");
        // Copy data
        log!("Copy code start");
        for p in elf
            .program_header_iter()
            .filter(|p| p.ph.ph_type() == ProgramType::LOAD)
        {
            let start: Address = (p.ph.vaddr() as usize).into();
            let aligned_start = Page::<Size4K, V>::align(start);
            let end = start + p.ph.filesz() as usize;
            let src = &elf_data[p.ph.offset() as usize] as *const u8;
            unsafe {
                log!("Copy code {:?}", start..end);
                let mut cursor = aligned_start;
                while cursor < end {
                    let f = translate(&mut *p4, Page::new(cursor));
                    let dst_start = Address::max(cursor, start);
                    let dst_end = Address::min(cursor + Size4K::BYTES, end);
                    let bytes = dst_end - dst_start;
                    let offset = dst_start - start;
                    let dst = f.start() + (dst_start - cursor);
                    ptr::copy_nonoverlapping::<u8>(src.add(offset), dst.as_mut_ptr(), bytes);

                    cursor += Size4K::BYTES;
                }
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
        log!(
            " - {:?} p={:?} v={:?} c={} end={:?}",
            desc.ty,
            desc.phys_start as *mut u8,
            desc.virt_start as *mut u8,
            desc.page_count,
            unsafe { (desc.phys_start as *mut u8).add((desc.page_count as usize) << 12) }
        );
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
    log!(
        "available_physical_memory_ranges @ {:?}",
        available_physical_memory_ranges.as_ptr_range()
    );
    return available_physical_memory_ranges;
}

fn gen_boot_info(device_tree: &'static [u8], init_fs: &'static [u8]) -> BootInfo {
    let fdt = Fdt::new(device_tree).unwrap();
    let uart = if let Some(node) = fdt.find_compatible(&["arm,pl011"]) {
        let mut addr = node.reg().unwrap().next().unwrap().starting_address as usize;
        if addr & 0xff000000 == 0x7e000000 {
            addr += 0x80000000
        }
        const UART: Address = Address::new(0xdead_0000_0000);
        map_kernel_page_4k(
            PageTable::<L4>::get(),
            Page::new(UART),
            Frame::new(addr.into()),
            PageFlags::device(),
        );
        Some(UART)
    } else {
        None
    };
    BootInfo {
        available_physical_memory: gen_available_physical_memory(),
        device_tree,
        uart,
        init_fs,
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

fn read_dtb(handle: Handle) -> &'static mut [u8] {
    // Try to get dtb path from command line args: dtb=...
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
    if let Some(dtb_path) = args
        .find(|x| x.starts_with("dtb="))
        .map(|x| x.strip_prefix("dtb=").unwrap())
    {
        log!("Load device tree from {}", dtb_path);
        return read_file(handle, dtb_path).leak();
    }
    // Try to load dtb from efi configuration table
    const GUID: Guid = Guid::from_values(
        0xb1b621d5,
        0xf19c,
        0x41a5,
        0x830b,
        [0xd9, 0x15, 0x2c, 0x69, 0xaa, 0xe0],
    );
    #[repr(C)]
    struct FDTHeader {
        magic: u32,
        totalsize: u32,
    }
    if let Some(cfg) = boot_system_table()
        .config_table()
        .iter()
        .find(|x| x.guid == GUID)
    {
        let size = unsafe { (*(cfg.address as *mut FDTHeader)).totalsize };
        let size = u32::from_le_bytes(size.to_be_bytes());
        let dtb = unsafe { slice::from_raw_parts_mut(cfg.address as *mut u8, size as _) };
        log!("Load device tree from EFI configuration table");
        return dtb;
    }

    log!("Config table:");
    for entry in boot_system_table().config_table() {
        log!(" - {} {:?}", entry.guid, entry.address);
    }

    panic!("Device tree not specified");
}

extern "C" fn launch_kernel_at_el1(
    start: extern "C" fn(&mut BootInfo) -> isize,
    boot_info: &mut BootInfo,
) -> ! {
    CNTHCTL_EL2.write(CNTHCTL_EL2::EL1PCEN::SET + CNTHCTL_EL2::EL1PCTEN::SET);
    CNTVOFF_EL2.set(0);
    HCR_EL2.write(HCR_EL2::RW::EL1IsAarch64);

    MAIR_EL1.write(
        // Attribute 1 - Cacheable normal DRAM.
        MAIR_EL1::Attr1_Normal_Outer::WriteBack_NonTransient_ReadWriteAlloc +
        MAIR_EL1::Attr1_Normal_Inner::WriteBack_NonTransient_ReadWriteAlloc +
        // Attribute 0 - Device.
        MAIR_EL1::Attr0_Device::nonGathering_nonReordering_EarlyWriteAck,
    );

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
            + TCR_EL1::EPD1::EnableTTBR1Walks, // + TCR_EL1::T0SZ.val(0x10)
                                               // + TCR_EL1::T1SZ.val(0x10)
    );
    TCR_EL1.set(TCR_EL1.get() | 0b101 << 32); // Intermediate Physical Address Size (IPS) = 0b101
    TCR_EL1.set(TCR_EL1.get() | 0x10 << 0); // TTBR0_EL1 memory size (T0SZ) = 0x10 ==> 2^(64 - T0SZ)
    TCR_EL1.set(TCR_EL1.get() | 0x10 << 16); // TTBR1_EL1 memory size (T1SZ) = 0x10 ==> 2^(64 - T1SZ)

    SCTLR_EL1.set((3 << 28) | (3 << 22) | (1 << 20) | (1 << 11)); // Disable MMU
    SCTLR_EL1.modify(SCTLR_EL1::M::Enable + SCTLR_EL1::C::Cacheable + SCTLR_EL1::I::Cacheable);
    SPSR_EL2.write(
        SPSR_EL2::D::Masked
            + SPSR_EL2::A::Masked
            + SPSR_EL2::I::Masked
            + SPSR_EL2::F::Masked
            + SPSR_EL2::M::EL1h,
    );

    log!("boot_info @ {:?}", boot_info as *const _);
    log!(
        "device_tree @ {:?}",
        (*boot_info).device_tree.as_ptr_range()
    );
    log!(
        "available_physical_memory @ {:?}",
        (*boot_info).available_physical_memory.as_ptr_range()
    );

    unsafe {
        {
            let buffer = &mut [0; 4096];
            boot_system_table()
                .unsafe_clone()
                .exit_boot_services(IMAGE.unwrap(), buffer)
                .unwrap_success();
        }
        asm! {
            "
                mov x0, #0xfffffff
                msr cpacr_el1, x0
                mov x0, sp
                msr sp_el1, x0
            ",
            in("x0") 0,
            in("x1") 0,
        }
        ELR_EL2.set(start as *const () as u64);
        asm! {
            "eret",
            in("x0") boot_info,
        }
    }
    unreachable!();
}

static mut BOOT_INFO: BootInfo = BootInfo {
    available_physical_memory: &[],
    device_tree: &[],
    init_fs: &[],
    uart: None,
};

#[no_mangle]
pub unsafe extern "C" fn efi_main(image: Handle, st: SystemTable<Boot>) -> Status {
    uefi_services::init(&st).expect_success("Failed to initialize utilities");
    BOOT_SYSTEM_TABLE = Some(st.unsafe_clone());
    IMAGE = Some(image);
    UEFILogger::init();
    log!("Hello, UEFI!");
    log!("CurrentEL {:?}", CurrentEL.get() >> 2);

    debug_assert_eq!(CurrentEL.get() >> 2, 2);

    log!("Loading kernel...");

    establish_el1_page_table();

    let kernel_elf = read_file(image, "sophon");
    let init_fs = read_file(image, "init.fs").leak();
    let dtb = read_dtb(image);
    let start = load_elf(&kernel_elf);

    log!("Starting kernel...");

    log!("DTB @ {:?}", dtb.as_ptr_range());

    BOOT_INFO = gen_boot_info(dtb, init_fs);

    launch_kernel_at_el1(start, &mut BOOT_INFO);
}

#[no_mangle]
pub extern "C" fn __chkstk() {}
