use goblin::elf::{Elf, program_header};
use crate::mm::*;
use crate::mm::heap_constants::*;
use core::iter::Step;
use cortex_a::regs::*;



pub fn exec_user(elf_data: &[u8]) -> ! {
    println!("exec_user");
    let elf = Elf::parse(elf_data).unwrap();
    println!("parsed");
    let entry: extern fn(isize, *const *const u8) = unsafe { ::core::mem::transmute(elf.header.e_entry) };
    println!("entry: {:?}", entry as *mut ());
    for p in elf.program_headers {
        if p.p_type == program_header::PT_LOAD {
            println!("pheader = {:?}", p);
            let start: Address = (p.p_vaddr as usize).into();
            let size = (p.p_memsz as usize + Size4K::MASK) / Size4K::SIZE;
            let end = start + (size << Size4K::LOG_SIZE);
            println!("{:?} {:?} {:?}", start, size, end);
            memory_map(start, size << Size4K::LOG_SIZE, PageFlags::USER | PageFlags::OUTER_SHARE | PageFlags::SMALL_PAGE | PageFlags::ACCESSED | PageFlags::PRESENT);
            let ptr: *mut u8 = start.as_ptr_mut();
            let mut cursor = start;
            while cursor < end {
                let offset = (cursor - start) as usize;
                if (p.p_offset as usize) + offset >= elf_data.len() {
                    break;
                }
                let v = elf_data[(p.p_offset as usize) + offset];
                // unsafe {
                //     cursor.store(v);
                // }
                if offset < p.p_filesz as usize {
                    // unsafe { println!("ptr {:?}", ptr.add(offset)); }
                    unsafe { *ptr.add(offset) = v };
                } else {
                    unsafe { *ptr.add(offset) = 0 };
                }
                cursor += 1;
            }
        }
    }
    // Alloc user stack
    memory_map(USER_STACK_START, USER_STACK_PAGES << Size4K::LOG_SIZE, PageFlags::USER | PageFlags::OUTER_SHARE | PageFlags::SMALL_PAGE | PageFlags::ACCESSED | PageFlags::PRESENT);
    // Enter usermode
    exit_to_user(entry, USER_STACK_START);
}

fn exit_to_user(entry: extern fn(_argc: isize, _argv: *const *const u8), sp: Address) -> ! {
    println!("ENTER USER MODE SP={:?}", USER_STACK_END);
    unsafe {
        asm! {
            "
            msr spsr_el1, $0
            msr elr_el1, $1
            msr sp_el0, $2
            eret
            "
            ::"r"(0), "r"(entry), "r"(USER_STACK_END.as_usize())
        }
    }
    unreachable!()
}