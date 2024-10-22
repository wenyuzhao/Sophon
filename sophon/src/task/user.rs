use core::{iter::Step, sync::atomic::Ordering};

use alloc::{ffi::CString, sync::Arc, vec::Vec};
use interrupt::UninterruptibleMutex;
use klib::proc::Process;
use memory::{
    address::{Address, V},
    page::{Page, PageSize, Size4K},
    page_table::{PageFlags, PageTable, L4},
};

use crate::arch::ArchContext;
use crate::{
    arch::{Arch, TargetArch},
    memory::{
        kernel::{KERNEL_MEMORY_MAPPER, KERNEL_MEMORY_RANGE},
        physical::PHYSICAL_MEMORY,
    },
    task::PROCESS_MANAGER,
};

use super::sched::SCHEDULER;

const USER_STACK_START: Address<V> = Address::new(0x111900000);
const USER_STACK_PAGES: usize = 4; // Too many???
const USER_STACK_SIZE: usize = USER_STACK_PAGES * Size4K::BYTES;

fn load_elf(page_table: &mut PageTable, elf_data: &[u8]) -> extern "C" fn(isize, *const *const u8) {
    let base = Address::<V>::from(0x200000);
    let entry = elf_loader::ELFLoader::load(elf_data, &mut |pages| {
        let start_page = Page::new(base);
        let num_pages = Page::steps_between(&pages.start, &pages.end).unwrap();
        for (i, _) in pages.enumerate() {
            let page = Page::<Size4K>::forward(start_page, i);
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
        start_page..Page::<Size4K>::forward(start_page, num_pages)
    })
    .unwrap();
    // log!("Entry: {:?}", entry.entry);
    unsafe { core::mem::transmute(entry.entry) }
}

pub fn initialize_user_space(proc: &Process, elf: &[u8]) -> extern "C" fn(isize, *const *const u8) {
    // Initialize addr space, page table and load ELF
    debug_assert_eq!(proc.id, PROCESS_MANAGER.current_proc().unwrap().id);
    // User page table
    let _guard = KERNEL_MEMORY_MAPPER.with_kernel_address_space();
    if proc.mem.has_user_page_table() {
        let page_table = proc.mem.get_page_table();
        page_table.release(&PHYSICAL_MEMORY);
    }
    let page_table = {
        let page_table = PageTable::alloc(&PHYSICAL_MEMORY);
        // Map kernel pages
        let kernel_memory = KERNEL_MEMORY_RANGE;
        let index = PageTable::<L4>::get_index(kernel_memory.start);
        debug_assert_eq!(index, PageTable::<L4>::get_index(kernel_memory.end - 1));
        page_table[index] = PageTable::get()[index].clone();
        // Set page table
        proc.mem.page_table.store(page_table, Ordering::SeqCst);
        proc.mem.has_user_page_table.store(true, Ordering::SeqCst);
        proc.mem.highwater.store(
            crate::memory::USER_SPACE_MEMORY_RANGE.start,
            Ordering::SeqCst,
        );
        PageTable::set(page_table);
        page_table
    };
    // Load ELF
    let entry = load_elf(page_table, elf);
    entry
}

pub fn setup_user_stack(page_table: &mut PageTable) -> Address {
    let tid = SCHEDULER.get_current_task_id().unwrap();
    let i = PROCESS_MANAGER
        .current_proc()
        .unwrap()
        .threads
        .lock_uninterruptible()
        .iter()
        .position(|t| *t == tid)
        .unwrap();
    // println!("User stack #{}", i);
    let user_stack_start = USER_STACK_START + i * USER_STACK_SIZE;
    for i in 0..USER_STACK_PAGES {
        let page = Step::forward(Page::<Size4K>::new(user_stack_start), i);
        let frame = PHYSICAL_MEMORY.acquire::<Size4K>().unwrap();
        let _guard = KERNEL_MEMORY_MAPPER.with_kernel_address_space();
        page_table.map(page, frame, PageFlags::user_stack_flags(), &PHYSICAL_MEMORY);
    }
    user_stack_start + USER_STACK_SIZE
}

pub fn prepare_args(
    args: &[CString],
    mut stack_top: Address,
) -> (isize, *const *const u8, Address) {
    let argc = args.len();
    let mut ptrs: Vec<*const u8> = Vec::with_capacity(argc);
    for arg in args {
        let buf = arg.to_bytes_with_nul();
        let ptr = stack_top - buf.len();
        unsafe { core::ptr::copy_nonoverlapping(buf.as_ptr(), ptr.as_mut_ptr(), buf.len()) };
        ptrs.push(ptr.as_ptr());
        stack_top = ptr;
    }
    for ptr in ptrs {
        stack_top = stack_top - size_of::<*const u8>();
        unsafe { stack_top.store(ptr) };
    }
    (argc as isize, stack_top.as_ptr::<*const u8>(), stack_top)
}

pub fn enter_usermode(
    entry: extern "C" fn(_argc: isize, _argv: *const *const u8),
    sp: Address,
    page_table: &mut PageTable,
    argc: isize,
    argv: *const *const u8,
) -> ! {
    unsafe { <TargetArch as Arch>::Context::enter_usermode(entry, sp, page_table, argc, argv) }
}

/// execve: Replace the current process with a new process.
pub fn exec(proc: Arc<Process>, elf: Vec<u8>, args: &[CString]) -> isize {
    assert_eq!(proc.id, PROCESS_MANAGER.current_proc().unwrap().id);
    if proc.threads.lock().len() != 1 {
        return -1;
    }
    let entry = initialize_user_space(&proc, &elf);
    let page_table = proc.mem.get_page_table();
    // Setup user stack
    let mut stack_top = super::user::setup_user_stack(page_table);
    // Prepare arguments
    let (argc, argv, s) = super::user::prepare_args(&args, stack_top);
    stack_top = s;
    core::mem::drop(proc);
    core::mem::drop(elf);
    // Enter usermode
    super::user::enter_usermode(entry, stack_top, page_table, argc, argv)
}
