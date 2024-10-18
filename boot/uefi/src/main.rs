#![no_std]
#![no_main]
#![feature(format_args_nl)]
#![feature(step_trait)]

extern crate alloc;
#[macro_use]
extern crate log;

use ::boot::BootInfo;
use alloc::vec;
use alloc::vec::Vec;
#[allow(unused)]
use core::arch::asm;
use core::iter::Step;
use core::{intrinsics::transmute, mem, ops::Range, slice};
use cortex_a::registers::*;
use device_tree::DeviceTree;
use elf_loader::ELFEntry;
use memory::address::*;
use memory::page::*;
use memory::page_table::*;
#[allow(unused)]
use tock_registers::interfaces::{ReadWriteable, Readable, Writeable};
use uefi::mem::memory_map::MemoryMap;
use uefi::proto::loaded_image::LoadedImage;
use uefi::proto::media::file::*;
use uefi::table::runtime::ResetType;
use uefi::{prelude::*, table::boot::*};
use uefi::{CStr16, Guid};

use uefi::boot;

use crate::uefi_logger::UEFILogger;

mod uefi_logger;

#[global_allocator]
static ALLOCATOR: uefi::allocator::Allocator = uefi::allocator::Allocator;

unsafe fn establish_el1_page_table() {
    let p4 = new_page4k().start().as_mut::<PageTable<L4>>();
    TTBR0_EL1.set(p4 as *const _ as u64);
    // Get physical address limit
    let mmap = boot::memory_map(MemoryType::LOADER_CODE).unwrap();
    let mut top = Address::<P>::ZERO;
    for desc in mmap.entries() {
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

fn new_page4k() -> Frame {
    let page = boot::allocate_pages(AllocateType::AnyPages, MemoryType::LOADER_CODE, 1)
        .unwrap()
        .as_ptr();
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
        let page = Page::new(Address::from((start + ((i as u64) << 12)) as usize));
        let frame = new_page4k();
        map_kernel_page_4k(p4, page, frame, PageFlags::kernel_code_flags_4k());
    }
}

fn load_elf(elf_data: &[u8]) -> ELFEntry {
    log!("Load kernel ELF");
    let kernel_base = Address::<V>::from(0xff0000000000usize);
    let entry = elf_loader::ELFLoader::load_with_address_translation(
        elf_data,
        &mut |pages| {
            let vaddr_start = kernel_base + pages.start.start();
            let num_pages = Page::steps_between(&pages.start, &pages.end).unwrap();
            let p4 = TTBR0_EL1.get() as *mut PageTable<L4>;
            map_kernel_pages_4k(unsafe { &mut *p4 }, vaddr_start.as_usize() as _, num_pages);
            let start_page = Page::new(vaddr_start);
            let end_page = Page::forward(start_page, num_pages);
            start_page..end_page
        },
        &|x| {
            let page = Page::containing(x);
            let offset = x - page.start();
            let p4 = TTBR0_EL1.get() as *mut PageTable<L4>;
            let p = unsafe { translate(&mut *p4, page).start() + offset };
            Address::new(p.as_usize())
        },
    )
    .unwrap();
    log!("Load kernel ELF done. entry @ {:?}", entry.entry);
    entry
}

fn gen_available_physical_memory() -> &'static [Range<Frame>] {
    let buffer = new_page4k();
    let mmap = boot::memory_map(MemoryType::LOADER_CODE).unwrap();
    let count = Frame::<Size4K>::BYTES / mem::size_of::<Range<Frame>>();
    let available_physical_memory_ranges: &'static mut [Range<Frame>] =
        unsafe { slice::from_raw_parts_mut(buffer.start().as_mut_ptr(), count) };
    let mut cursor = 0;
    for desc in mmap.entries() {
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
    let uart = {
        let devtree = DeviceTree::new(device_tree).unwrap();
        let node = devtree.compatible("arm,pl011").unwrap();
        let addr = node.translate(node.regs().unwrap().next().unwrap().start);
        const UART: Address = Address::new(0xdead_0000_0000);
        map_kernel_page_4k(
            PageTable::<L4>::get(),
            Page::new(UART),
            Frame::new(addr),
            PageFlags::device(),
        );
        Some(UART)
    };
    BootInfo {
        available_physical_memory: gen_available_physical_memory(),
        device_tree,
        uart,
        init_fs,
        shutdown: Some(shutdown),
    }
}

fn read_file(handle: Handle, path: &str) -> Vec<u8> {
    let mut sfs =
        boot::get_image_file_system(handle).expect("Cannot open `SimpleFileSystem` protocol");
    let mut directory = sfs.open_volume().unwrap();
    let mut data = [0u16; 512];
    let filename = CStr16::from_str_with_buf(path, &mut data).unwrap();

    let file = directory
        .open(filename, FileMode::Read, FileAttribute::empty())
        .unwrap()
        .into_type()
        .unwrap();
    if let FileType::Regular(mut file) = file {
        let mut buffer = vec![];
        let mut buf = vec![0; 4096];
        let mut total_size = 0usize;
        loop {
            let size = file.read(&mut buf).unwrap();
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
    let loaded_image = boot::open_protocol_exclusive::<LoadedImage>(boot::image_handle()).unwrap();
    let options = loaded_image.load_options_as_bytes();
    if let Some(options) = options {
        let mut args = core::str::from_utf8(options).unwrap().split(" ");
        if let Some(dtb_path) = args
            .find(|x| x.starts_with("dtb="))
            .map(|x| x.strip_prefix("dtb=").unwrap())
        {
            log!("Load device tree from {}", dtb_path);
            return read_file(handle, dtb_path).leak();
        }
    }
    // Try to load dtb from efi configuration table
    const FDT_TABLE_GUID: Guid = uefi::guid!("b1b621d5-f19c-41a5-830b-d9152c69aae0");
    #[repr(C)]
    struct FDTHeader {
        magic: u32,
        totalsize: u32,
    }
    let result = uefi::system::with_config_table(|entries| {
        if let Some(cfg) = entries.iter().find(|x| x.guid == FDT_TABLE_GUID) {
            let size = unsafe { (*(cfg.address as *mut FDTHeader)).totalsize };
            let size = u32::from_le_bytes(size.to_be_bytes());
            let dtb = unsafe { slice::from_raw_parts_mut(cfg.address as *mut u8, size as _) };
            log!("Load device tree from EFI configuration table");
            return Some(dtb);
        }
        None
    });
    if let Some(dtb) = result {
        return dtb;
    }
    uefi::system::with_config_table(|entries| {
        log!("Config table:");
        for entry in entries {
            log!(" - {} {:?}", entry.guid, entry.address);
        }
    });

    panic!("Device tree not specified");
}

#[cfg(target_arch = "x86_64")]
extern "C" fn launch_kernel(
    _start: extern "C" fn(&mut BootInfo) -> isize,
    _boot_info: &mut BootInfo,
) -> ! {
    unimplemented!()
}

#[allow(unused)]
unsafe extern "C" fn kernel_entry(
    start: extern "C" fn(&mut BootInfo) -> isize,
    boot_info: &'static mut BootInfo,
) -> ! {
    if let Some(init_array) = INIT_ARRAY {
        for init in init_array {
            init();
        }
    }
    start(boot_info);
    loop {}
}

#[cfg(target_arch = "aarch64")]
extern "C" fn launch_kernel(
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
            let _mmap = boot::exit_boot_services(MemoryType::LOADER_CODE);
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
        ELR_EL2.set(kernel_entry as *const () as u64);
        asm! {
            "eret",
            in("x0") start,
            in("x1") boot_info,
        }
    }
    unreachable!();
}

static mut BOOT_INFO: BootInfo = BootInfo {
    available_physical_memory: &[],
    device_tree: &[],
    init_fs: &[],
    uart: None,
    shutdown: None,
};

static mut INIT_ARRAY: Option<&'static [extern "C" fn()]> = None;

extern "C" fn shutdown() -> ! {
    uefi::runtime::reset(ResetType::SHUTDOWN, Status::SUCCESS, None);
}

#[entry]
pub unsafe fn main() -> Status {
    uefi::helpers::init().expect("Failed to initialize uefi");

    let image = boot::image_handle();

    UEFILogger::init();
    log!("Hello, UEFI!");
    log!("CurrentEL {:?}", CurrentEL.get() >> 2);

    debug_assert_eq!(CurrentEL.get() >> 2, 2);

    log!("Loading kernel...");

    // let mut config_entries = st.config_table().iter();
    // let rsdp_addr = config_entries
    //     .find(|entry| matches!(entry.guid, cfg::ACPI2_GUID))
    //     .map(|entry| entry.address);
    // log!("RSDP @ {:?}", rsdp_addr);

    establish_el1_page_table();

    let kernel_elf = read_file(image, "sophon");
    let init_fs = read_file(image, "init.fs").leak();
    let dtb = read_dtb(image);
    let entry = load_elf(&kernel_elf);

    log!("Starting kernel...");

    log!("DTB @ {:?}", dtb.as_ptr_range());

    BOOT_INFO = gen_boot_info(dtb, init_fs);
    INIT_ARRAY = mem::transmute(entry.init_array);

    #[allow(static_mut_refs)]
    launch_kernel(mem::transmute(entry.entry), &mut BOOT_INFO);
}

#[no_mangle]
pub extern "C" fn __chkstk() {}
