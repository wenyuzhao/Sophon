use core::marker::PhantomData;
use super::KernelTask;
use crate::AbstractKernel;
use crate::arch::*;
use crate::memory::*;
use proton::memory::*;
use elf_rs::*;




const PT_LOAD: u32 = 1;


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
            for p in elf.program_headers() {
                if p.p_type == PT_LOAD {
                    // println!("pheader = {:?}", p);
                    let start: Address = (p.p_vaddr as usize).into();
                    let size = (p.p_memsz as usize + Size4K::MASK) / Size4K::SIZE;
                    let end = start + (size << Size4K::LOG_SIZE);
                    debug!(K: "Map {:?} {:?} {:?}", start, size, end);
                    memory_map::<K>(start, size << Size4K::LOG_SIZE, PageFlags::user_code_flags()).unwrap();
                    let ptr: *mut u8 = start.as_ptr_mut();
                    let mut cursor = start;
                    while cursor < end {
                        let offset = (cursor - start) as usize;
                        if (p.p_offset as usize) + offset >= self.elf_data.len() {
                            break;
                        }
                        let v = self.elf_data[(p.p_offset as usize) + offset];
                        if offset < p.p_filesz as usize {
                            unsafe { *ptr.add(offset) = v };
                        } else {
                            unsafe { *ptr.add(offset) = 0 };
                        }
                        cursor += 1;
                    }
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
