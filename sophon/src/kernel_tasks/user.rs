use super::KernelTask;
use crate::arch::*;
use crate::memory::kernel::KERNEL_MEMORY_MAPPER;
use crate::memory::kernel::KERNEL_MEMORY_RANGE;
use crate::memory::physical::PHYSICAL_MEMORY;
use crate::task::Proc;
use alloc::vec::Vec;
use core::iter::Step;
use core::ptr;
use elf_rs::*;
use memory::address::*;
use memory::page::*;
use memory::page_table::{PageFlags, PageFlagsExt, PageTable, L4};

const USER_STACK_START: Address<V> = Address::new(0x111900000);
const USER_STACK_PAGES: usize = 4; // Too many???
const USER_STACK_SIZE: usize = USER_STACK_PAGES * Size4K::BYTES;
const USER_STACK_END: Address<V> = Address::new(USER_STACK_START.as_usize() + USER_STACK_SIZE);

pub struct UserTask {
    elf_data: Vec<u8>,
}

impl UserTask {
    pub fn new(elf_data: Vec<u8>) -> Self {
        Self { elf_data }
    }

    fn setup_user_pagetable() -> &'static mut PageTable {
        let page_table = PageTable::alloc(&PHYSICAL_MEMORY);
        // Map kernel pages
        let kernel_memory = KERNEL_MEMORY_RANGE;
        let index = PageTable::<L4>::get_index(kernel_memory.start);
        debug_assert_eq!(index, PageTable::<L4>::get_index(kernel_memory.end - 1));
        page_table[index] = PageTable::get()[index].clone();
        Proc::current().set_page_table(unsafe { &mut *(page_table as *mut _) });
        page_table
    }

    fn setup_user_stack(page_table: &mut PageTable) {
        for i in 0..USER_STACK_PAGES {
            let page = Step::forward(Page::<Size4K>::new(USER_STACK_START), i);
            let frame = PHYSICAL_MEMORY.acquire::<Size4K>().unwrap();
            let _guard = KERNEL_MEMORY_MAPPER.with_kernel_page_table();
            page_table.map(page, frame, PageFlags::user_stack_flags(), &PHYSICAL_MEMORY);
        }
    }

    fn load_elf(&self, page_table: &mut PageTable) -> extern "C" fn(isize, *const *const u8) {
        let elf = Elf::from_bytes(&self.elf_data).unwrap();
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
                let _kernel_page_table = KERNEL_MEMORY_MAPPER.with_kernel_page_table();
                page_table.map(
                    page,
                    frame,
                    PageFlags::user_code_flags_4k(),
                    &PHYSICAL_MEMORY,
                );
            }
            TargetArch::set_current_page_table(Frame::new(page_table.into()));
            // Copy data
            for p in elf
                .program_header_iter()
                .filter(|p| p.ph.ph_type() == ProgramType::LOAD)
            {
                let start: Address = (p.ph.vaddr() as usize).into();
                let bytes = p.ph.filesz() as usize;
                let offset = p.ph.offset() as usize;
                unsafe {
                    ptr::copy_nonoverlapping::<u8>(
                        &self.elf_data[offset],
                        start.as_mut_ptr(),
                        bytes,
                    );
                }
            }
            if let Some(bss) = elf.lookup_section(".bss") {
                let start = Address::<V>::from(bss.sh.addr() as usize);
                let size = bss.sh.size() as usize;
                unsafe {
                    ptr::write_bytes::<u8>(start.as_mut_ptr(), 0, size);
                }
            }
            TargetArch::clear_cache(vaddr_start..vaddr_end);
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
        let page_table = Self::setup_user_pagetable();
        log!("User page-table created");
        let entry = self.load_elf(page_table);
        log!("ELF File loaded");
        Self::setup_user_stack(page_table);
        log!("User stack created");
        log!(
            "Start to enter usermode: {:?}",
            crate::task::Task::current().id
        );
        // Enter usermode
        unsafe { <TargetArch as Arch>::Context::enter_usermode(entry, USER_STACK_END, page_table) }
    }
}
