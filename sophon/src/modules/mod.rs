use core::ptr;

use alloc::{borrow::ToOwned, string::String, vec::Vec};
use elf_rs::{Elf, ElfFile, ProgramType};
use hashbrown::HashMap;
use kernel_module::KernelServiceWrapper;
use memory::{
    address::{Address, V},
    page::{Page, PageResource, PageSize, Size2M, Size4K},
};
use spin::{Lazy, Mutex};

use crate::memory::kernel::KERNEL_HEAP;

fn load_elf(elf_data: &[u8]) -> extern "C" fn(kernel_module::KernelServiceWrapper) -> usize {
    let elf = Elf::from_bytes(elf_data).unwrap();
    if let Elf::Elf64(elf) = elf {
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
            .filter(|p| p.ph_type() == ProgramType::LOAD)
        {
            let start: Address = (p.vaddr() as usize).into();
            let end = start + (p.filesz() as usize);
            update_load_range(start, end);
        }
        if let Some(bss) = elf.lookup_section(".bss".as_bytes()) {
            let start = Address::<V>::from(bss.addr() as usize);
            let end = start + bss.size() as usize;
            update_load_range(start, end);
        }
        let vaddr_start = Page::<Size4K>::align(load_start.unwrap());
        let vaddr_end = load_end.unwrap().align_up(Size2M::BYTES);
        let num_pages = (vaddr_end - vaddr_start) >> Page::<Size2M>::LOG_BYTES;
        let pages = KERNEL_HEAP.acquire_pages::<Size2M>(num_pages).unwrap();
        let base_address = pages.start.start();
        // Copy data
        for p in elf
            .program_header_iter()
            .filter(|p| p.ph_type() == ProgramType::LOAD)
        {
            let start: Address = (p.vaddr() as usize).into();
            let bytes = p.filesz() as usize;
            let offset = p.offset() as usize;
            unsafe {
                ptr::copy_nonoverlapping::<u8>(
                    &elf_data[offset],
                    (base_address + (start - vaddr_start)).as_mut_ptr(),
                    bytes,
                );
            }
        }
        if let Some(bss) = elf.lookup_section(".bss".as_bytes()) {
            let start = base_address + bss.offset() as usize;
            let size = bss.size() as usize;
            unsafe {
                ptr::write_bytes::<u8>(start.as_mut_ptr(), 0, size);
            }
        }
        memory::cache::flush_cache(base_address..base_address + (vaddr_end - vaddr_start));
        let entry = unsafe {
            ::core::mem::transmute(
                base_address + elf.elf_header().entry_point() as usize - vaddr_start.as_usize(),
            )
        };
        entry
    } else {
        unimplemented!("elf32 is not supported")
    }
}

struct KernelModule {
    name: String,
    init: extern "C" fn(kernel_module::KernelServiceWrapper) -> usize,
    deinit: Option<extern "C" fn()>,
    elf: Vec<u8>,
}

static MODULES: Lazy<Mutex<HashMap<String, KernelModule>>> = Lazy::new(Default::default);

pub fn register(name: &str, elf: Vec<u8>) {
    let init = {
        let mut modules = MODULES.lock();
        if modules.contains_key(name) {
            return;
        }
        let init = load_elf(&elf);
        modules.insert(
            name.to_owned(),
            KernelModule {
                name: name.to_owned(),
                init,
                deinit: None,
                elf,
            },
        );
        init
    };
    init(KernelServiceWrapper::from_service(&KERNEL_SERVICE));
}

pub struct KernelService;

impl kernel_module::KernelService for KernelService {
    fn log(&self, s: &str) {
        log!("{}", s);
    }
}

static KERNEL_SERVICE: KernelService = KernelService;
