use core::iter::Step;

use super::KernelTask;
use crate::arch::*;
use crate::memory::physical::PhysicalPageResource;
use crate::memory::physical::PHYSICAL_PAGE_RESOURCE;
// use crate::page_table::PageFlags;
// use crate::page_table::PageTable;
// use crate::page_table::L4;
use crate::utils::address::*;
use crate::utils::page::*;
use elf_rs::*;

const USER_STACK_START: Address<V> = Address::new(0x111900000);
const USER_STACK_PAGES: usize = 4; // Too many???
const USER_STACK_SIZE: usize = USER_STACK_PAGES * Size4K::BYTES;
const USER_STACK_END: Address<V> = Address::new(USER_STACK_START.as_usize() + USER_STACK_SIZE);

pub struct UserTask {
    elf_data: &'static [u8],
}

impl UserTask {
    pub fn new(elf_data: &'static [u8]) -> Self {
        Self { elf_data }
    }

    fn create_user_pagetable() {}

    fn load_elf(&self) -> extern "C" fn(isize, *const *const u8) {
        let elf = Elf::from_bytes(&self.elf_data).unwrap();
        if let Elf::Elf64(elf) = elf {
            log!("Parsed ELF file");
            let entry: extern "C" fn(isize, *const *const u8) =
                unsafe { ::core::mem::transmute(elf.header().entry_point()) };
            log!("Entry: {:?}", entry as *mut ());
            let mut load_start = None;
            let mut load_end = None;
            for p in elf
                .program_header_iter()
                .filter(|p| p.ph.ph_type() == ProgramType::LOAD)
            {
                log!("{:?}", p.ph);
                let start: Address = (p.ph.vaddr() as usize).into();
                let end = start + (p.ph.filesz() as usize);
                match (load_start, load_end) {
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
                }
            }
            log!(
                "vaddr: {:?} .. {:?}",
                load_start.unwrap(),
                load_end.unwrap()
            );
            let vaddr_start = Page::<Size4K>::align(load_start.unwrap());
            let vaddr_end = load_end.unwrap().align_up(Size4K::BYTES);
            let pages = (vaddr_end - vaddr_start) >> Page::<Size4K>::LOG_BYTES;
            let frames = PHYSICAL_PAGE_RESOURCE
                .lock()
                .acquire::<Size4K>(pages)
                .unwrap();
            let mut page = Page::<Size4K>::new(vaddr_start);
            unimplemented!();
            // let pt = PageTable::<L4>::get(false);
            // for f in frames {
            //     pt.map(page, f, PageFlags::user_code_flags_4k());
            //     page = Step::forward(page, 1);
            // }
            // Copy data
            for p in elf
                .program_header_iter()
                .filter(|p| p.ph.ph_type() == ProgramType::LOAD)
            {
                let start: Address = (p.ph.vaddr() as usize).into();
                let bytes = p.ph.filesz() as usize;
                let offset = p.ph.offset() as usize;
                for i in 0..bytes {
                    let v = self.elf_data[offset + i];
                    unsafe {
                        (start + i).store(v);
                    }
                }
            }
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
        let entry = self.load_elf();
        log!("ELF File loaded");
        // Allocate user stack
        // memory_map::<K>(
        //     USER_STACK_START,
        //     USER_STACK_PAGES << Size4K::LOG_SIZE,
        //     PageFlags::user_stack_flags(),
        // )
        // .unwrap();
        {
            unimplemented!()
            // let frames = PHYSICAL_PAGE_RESOURCE
            //     .lock()
            //     .acquire::<Size4K>(USER_STACK_PAGES)
            //     .unwrap();
            // let mut page = Page::<Size4K>::new(USER_STACK_START);
            // let pt = PageTable::<L4>::get(false);
            // for f in frames {
            //     pt.map(page, f, PageFlags::page_table_flags());
            //     page = Step::forward(page, 1);
            // }
        }
        log!("Stack memory mapped");
        // <K::Arch as AbstractArch>::Interrupt::disable();
        log!(
            "Start to enter usermode: {:?}",
            crate::task::Task::current().map(|t| t.id())
        );
        // Enter usermode
        unsafe { <TargetArch as Arch>::Context::enter_usermode(entry, USER_STACK_END) }
    }
}
