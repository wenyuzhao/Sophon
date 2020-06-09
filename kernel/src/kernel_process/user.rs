use core::marker::PhantomData;
use super::KernelTask;
use crate::AbstractKernel;
use crate::arch::*;
use crate::memory::*;
use proton::memory::*;
use elf_rs::*;



const USER_STACK_START: Address<V> = Address::new(0x111900000);
const USER_STACK_PAGES: usize = 4; // Too many???
const USER_STACK_SIZE: usize = USER_STACK_PAGES * Size4K::SIZE;
const USER_STACK_END: Address<V> = Address::new(USER_STACK_START.as_usize() + USER_STACK_SIZE);

pub struct UserTask<K: AbstractKernel> {
    phantom: PhantomData<K>,
    elf_data: &'static [u8],
}

impl <K: AbstractKernel> UserTask<K> {
    pub fn new(elf_data: &'static [u8]) -> Self {
        Self {
            phantom: PhantomData,
            elf_data,
        }
    }

    fn load_elf(&self) -> extern fn(isize, *const *const u8) {
        let elf = Elf::from_bytes(&self.elf_data).unwrap();
        if let Elf::Elf64(elf) = elf {
            debug!(K: "Parsed ELF file");
            let entry: extern fn(isize, *const *const u8) = unsafe { ::core::mem::transmute(elf.header().entry_point()) };
            debug!(K: "Entry: {:?}", entry as *mut ());
            let mut load_start = None;
            let mut load_end = None;
            for p in elf.program_header_iter().filter(|p| p.ph.ph_type() == ProgramType::LOAD) {
                debug!(K: "{:?}", p.ph);
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
            debug!(K: "vaddr: {:?} .. {:?}", load_start.unwrap(), load_end.unwrap());
            let vaddr_start = Page::<Size4K>::align(load_start.unwrap());
            let vaddr_end = Page::<Size4K>::align_up(load_end.unwrap());
            memory_map::<K>(vaddr_start, vaddr_end - vaddr_start, PageFlags::user_code_flags()).unwrap();
            // Copy data
            for p in elf.program_header_iter().filter(|p| p.ph.ph_type() == ProgramType::LOAD) {
                let start: Address = (p.ph.vaddr() as usize).into();
                let bytes = p.ph.filesz() as usize;
                let offset = p.ph.offset() as usize;
                for i in 0..bytes {
                    let v = self.elf_data[offset + i];
                    unsafe { (start + i).store(v); }
                }
            }
            entry
        } else {
            unimplemented!("elf32 is not supported")
        }
    }
}

impl <K: AbstractKernel> KernelTask for UserTask<K> {
    fn run(&mut self) -> ! {
        debug!(K: "User task start (kernel)");
        debug!(K: "Execute user program");
        let entry = self.load_elf();
        debug!(K: "ELF File loaded");
        // Allocate user stack
        memory_map::<K>(USER_STACK_START, USER_STACK_PAGES << Size4K::LOG_SIZE, PageFlags::user_stack_flags()).unwrap();
        debug!(K: "Stack memory mapped");
        // <K::Arch as AbstractArch>::Interrupt::disable();
        debug!(K: "Start to enter usermode: {:?}", crate::task::Task::<K>::current().map(|t| t.id()));
        // Enter usermode
        unsafe {
            <K::Arch as AbstractArch>::Context::enter_usermode(entry, USER_STACK_END);
        }
    }
}
