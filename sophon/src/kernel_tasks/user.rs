use super::KernelTask;
use crate::arch::*;
use crate::memory::kernel::KERNEL_MEMORY_MAPPER;
use crate::memory::physical::PHYSICAL_MEMORY;
use crate::task::Proc;
use crate::task::Task;
use alloc::vec::Vec;
use core::iter::Step;
use core::ptr;
use elf_rs::*;
use interrupt::UninterruptibleMutex;
use memory::address::*;
use memory::page::*;
use memory::page_table::{PageFlags, PageFlagsExt, PageTable};

const USER_STACK_START: Address<V> = Address::new(0x111900000);
const USER_STACK_PAGES: usize = 4; // Too many???
const USER_STACK_SIZE: usize = USER_STACK_PAGES * Size4K::BYTES;

pub struct UserTask {
    elf_data: Option<Vec<u8>>,
    entry: Option<*const extern "C" fn()>,
}

impl UserTask {
    pub fn new(entry: *const extern "C" fn()) -> Self {
        Self {
            elf_data: None,
            entry: Some(entry),
        }
    }

    pub fn new_with_elf(elf_data: Vec<u8>) -> Self {
        Self {
            elf_data: Some(elf_data),
            entry: None,
        }
    }

    fn setup_user_stack(page_table: &mut PageTable) -> Address {
        let tid = Task::current().id;
        let i = Proc::current()
            .threads
            .lock_uninterruptible()
            .iter()
            .position(|t| *t == tid)
            .unwrap();
        println!("User stack #{}", i);
        let user_stack_start = USER_STACK_START + i * USER_STACK_SIZE;
        for i in 0..USER_STACK_PAGES {
            let page = Step::forward(Page::<Size4K>::new(user_stack_start), i);
            let frame = PHYSICAL_MEMORY.acquire::<Size4K>().unwrap();
            let _guard = KERNEL_MEMORY_MAPPER.with_kernel_address_space();
            page_table.map(page, frame, PageFlags::user_stack_flags(), &PHYSICAL_MEMORY);
        }
        user_stack_start + USER_STACK_SIZE
    }

    fn load_elf(&self, page_table: &mut PageTable) -> extern "C" fn(isize, *const *const u8) {
        let elf_data: &[u8] = self.elf_data.as_ref().unwrap();
        let elf = Elf::from_bytes(elf_data).unwrap();
        if let Elf::Elf64(elf) = elf {
            log!("Parsed ELF file");
            let entry: extern "C" fn(isize, *const *const u8) =
                unsafe { ::core::mem::transmute(elf.header().entry_point()) };
            log!("Entry: {:?}", entry as *mut ());
            let mut load_start = None;
            let mut load_end = None;
            let mut update_load_range = |start: Address, end: Address| match (load_start, load_end)
            {
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
                log!("{:?}", p.ph);
                let start: Address = (p.ph.vaddr() as usize).into();
                let end = start + (p.ph.filesz() as usize);
                update_load_range(start, end);
            }
            if let Some(bss) = elf.lookup_section(".bss") {
                log!("{:?}", bss);
                let start = Address::<V>::from(bss.sh.addr() as usize);
                let end = start + bss.sh.size() as usize;
                update_load_range(start, end);
            }
            log!(
                "vaddr: {:?} .. {:?}",
                load_start.unwrap(),
                load_end.unwrap()
            );
            let vaddr_start = Page::<Size4K>::align(load_start.unwrap());
            let vaddr_end = load_end.unwrap().align_up(Size4K::BYTES);
            let pages = (vaddr_end - vaddr_start) >> Page::<Size4K>::LOG_BYTES;
            let start_page = Page::<Size4K>::new(vaddr_start);
            for i in 0..pages {
                let page = Step::forward(start_page, i);
                let frame = PHYSICAL_MEMORY.acquire::<Size4K>().unwrap();
                let _kernel_page_table = KERNEL_MEMORY_MAPPER.with_kernel_address_space();
                page_table.map(
                    page,
                    frame,
                    PageFlags::user_code_flags_4k(),
                    &PHYSICAL_MEMORY,
                );
            }
            PageTable::set(page_table);
            // Copy data
            for p in elf
                .program_header_iter()
                .filter(|p| p.ph.ph_type() == ProgramType::LOAD)
            {
                let start: Address = (p.ph.vaddr() as usize).into();
                let bytes = p.ph.filesz() as usize;
                let offset = p.ph.offset() as usize;
                unsafe {
                    ptr::copy_nonoverlapping::<u8>(&elf_data[offset], start.as_mut_ptr(), bytes);
                }
            }
            if let Some(bss) = elf.lookup_section(".bss") {
                let start = Address::<V>::from(bss.sh.addr() as usize);
                let size = bss.sh.size() as usize;
                unsafe {
                    ptr::write_bytes::<u8>(start.as_mut_ptr(), 0, size);
                }
            }
            memory::cache::flush_cache(vaddr_start..vaddr_end);
            entry
        } else {
            unimplemented!("elf32 is not supported")
        }
    }
}

impl KernelTask for UserTask {
    fn run(&mut self) -> ! {
        log!("User task start (kernel)");
        log!("Execute user program");
        let proc = Proc::current();
        if Proc::current().threads.lock().len() == 1 {
            log!("Initialze user space process");
            proc.initialize_user_space();
            let page_table = proc.get_page_table();
            log!("Load ELF");
            let entry = self.load_elf(page_table);
            log!("Setup stack");
            let stack_top = Self::setup_user_stack(page_table);
            log!("Enter usermode");
            unsafe { <TargetArch as Arch>::Context::enter_usermode(entry, stack_top, page_table) }
        } else {
            let page_table = proc.get_page_table();
            log!("Setup stack");
            let stack_top = Self::setup_user_stack(page_table);
            let entry = self.entry.unwrap();
            log!("Enter usermode");
            unsafe {
                <TargetArch as Arch>::Context::enter_usermode(
                    core::mem::transmute(entry),
                    stack_top,
                    page_table,
                )
            }
        }
    }
}
