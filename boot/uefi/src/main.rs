#![no_std]
#![no_main]
#![feature(format_args_nl)]
#![feature(core_intrinsics)]
#![feature(step_trait)]

extern crate alloc;
#[macro_use]
extern crate log;

use alloc::vec;
use alloc::vec::Vec;
use boot::BootInfo;
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
use uefi::proto::loaded_image::LoadedImage;
use uefi::proto::media::file::*;
use uefi::table::runtime::ResetType;
use uefi::{prelude::*, table::boot::*};
use uefi::{CStr16, Guid};

use crate::uefi_logger::UEFILogger;

mod smp;
mod uefi_logger;

static FORCE_NUM_CPUS: spin::Lazy<Option<usize>> =
    spin::Lazy::new(|| option_env!("SOPHON_CPUS").map(|s| s.parse().unwrap()));

static mut BOOT_SYSTEM_TABLE: Option<SystemTable<Boot>> = None;
static mut RUNTIME_SERVICES: Option<&'static RuntimeServices> = None;
static mut IMAGE: Option<Handle> = None;

unsafe fn establish_el1_page_table() -> &'static mut PageTable {
    let p4 = new_page4k().start().as_mut::<PageTable<L4>>();
    PageTable::<L4>::set(p4);
    // Get physical address limit
    let mut buffer = [0u8; 4096];
    let (_, descriptors) = boot_system_table()
        .boot_services()
        .memory_map(&mut buffer)
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
    p4
}

fn boot_system_table() -> &'static mut SystemTable<Boot> {
    unsafe { BOOT_SYSTEM_TABLE.as_mut().unwrap() }
}

fn new_page4k() -> Frame {
    let page = boot_system_table()
        .boot_services()
        .allocate_pages(AllocateType::AnyPages, MemoryType::LOADER_CODE, 1)
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
    unsafe {
        INIT_ARRAY = mem::transmute(entry.init_array);
    }
    log!("Load kernel ELF done. entry @ {:?}", entry.entry);
    entry
}

fn gen_available_physical_memory() -> &'static [Range<Frame>] {
    let bt = boot_system_table();
    let buffer = new_page4k();
    let (_, descriptors) = bt
        .boot_services()
        .memory_map(unsafe { buffer.start().as_mut::<[u8; 4096]>() })
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

fn get_num_cpus(device_tree: &DeviceTree) -> usize {
    let max_num_cpus = device_tree.cpus().count();
    if let Some(num_cpus) = *FORCE_NUM_CPUS {
        assert!(0 < num_cpus && num_cpus <= max_num_cpus);
        num_cpus
    } else {
        max_num_cpus
    }
}

fn gen_boot_info(
    device_tree: &DeviceTree,
    num_cpus: usize,
    init_fs: &'static [u8],
    dtb: &'static [u8],
) -> BootInfo {
    let uart = {
        let node = device_tree.compatible("arm,pl011").unwrap();
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
        device_tree: dtb,
        uart,
        init_fs,
        shutdown: Some(shutdown),
        start_ap: if num_cpus != 1 {
            Some(smp::start_ap)
        } else {
            None
        },
        num_cpus,
    }
}

fn read_file(handle: Handle, path: &str) -> Vec<u8> {
    let sfs = unsafe {
        &mut *boot_system_table()
            .boot_services()
            .get_image_file_system(handle)
            .expect("Cannot open `SimpleFileSystem` protocol")
            .interface
            .get()
    };
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
    let loaded_image = unsafe {
        &*boot_system_table()
            .boot_services()
            .open_protocol::<LoadedImage>(
                OpenProtocolParams {
                    handle: handle,
                    agent: handle,
                    controller: None,
                },
                OpenProtocolAttributes::Exclusive,
            )
            .expect("Failed to retrieve `LoadedImage` protocol from handle")
            .interface
            .get()
    };
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
    const GUID: Guid = Guid::from_values(
        0xb1b621d5,
        0xf19c,
        0x41a5,
        0x830b,
        u64::from_be_bytes([0, 0, 0xd9, 0x15, 0x2c, 0x69, 0xaa, 0xe0]),
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

#[allow(unused)]
unsafe extern "C" fn kernel_entry(
    start: extern "C" fn(&mut BootInfo, usize) -> isize,
    boot_info: &'static mut BootInfo,
    core: usize,
) -> ! {
    if core == 0 {
        if let Some(init_array) = INIT_ARRAY {
            for init in init_array {
                init();
            }
        }
    }
    start(boot_info, core);
    loop {}
}

static mut BOOT_INFO: BootInfo = BootInfo {
    available_physical_memory: &[],
    device_tree: &[],
    init_fs: &[],
    uart: None,
    shutdown: None,
    start_ap: None,
    num_cpus: 0,
};

static mut INIT_ARRAY: Option<&'static [extern "C" fn()]> = None;

extern "C" fn shutdown() -> ! {
    unsafe {
        RUNTIME_SERVICES
            .as_ref()
            .unwrap()
            .reset(ResetType::Shutdown, Status::SUCCESS, None);
    }
}

#[no_mangle]
pub unsafe extern "C" fn efi_main(image: Handle, mut st: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut st).expect("Failed to initialize utilities");
    BOOT_SYSTEM_TABLE = Some(st.unsafe_clone());
    RUNTIME_SERVICES = Some(BOOT_SYSTEM_TABLE.as_ref().unwrap().runtime_services());
    IMAGE = Some(image);
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

    let p4 = establish_el1_page_table();
    let kernel_elf = read_file(image, "sophon");
    let init_fs = read_file(image, "init.fs").leak();
    let dtb = read_dtb(image);
    let entry = load_elf(&kernel_elf);

    log!("Starting kernel...");

    // Prepare cores and boot-info for kernel
    {
        log!("Device tree @ {:?}", dtb.as_ptr_range());
        let devtree = DeviceTree::new(dtb).unwrap();
        let num_cpus = get_num_cpus(&devtree);
        smp::boot_and_prepare_ap(num_cpus, p4.into(), mem::transmute(entry.entry));
        BOOT_INFO = gen_boot_info(&devtree, num_cpus, init_fs, dtb);
    }
    // Start kernel
    smp::start_core(0, mem::transmute(entry.entry), &mut BOOT_INFO);
}

#[no_mangle]
pub extern "C" fn __chkstk() {}
