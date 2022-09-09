use super::MMState;
use crate::arch::*;
use crate::memory::kernel::KERNEL_MEMORY_MAPPER;
use crate::memory::kernel::KERNEL_MEMORY_RANGE;
use crate::memory::physical::PHYSICAL_MEMORY;
use crate::modules::PROCESS_MANAGER;
use alloc::ffi::CString;
use alloc::sync::Arc;
use alloc::vec::Vec;
use atomic::Ordering;
use core::arch::asm;
use core::iter::Step;
use core::mem::size_of;
use core::mem::transmute;
use core::ptr::copy_nonoverlapping;
use interrupt::UninterruptibleMutex;
use memory::address::*;
use memory::page::*;
use memory::page_table::L4;
use memory::page_table::{PageFlags, PageTable};
use proc::Proc;
use proc::Runnable;

/// The idle task.
///
/// The task scheduler should schedule this task when no other task is ready.
pub struct Idle;

impl Runnable for Idle {
    fn run(&mut self) -> ! {
        loop {
            unsafe {
                asm!("wfe");
            }
        }
    }
}

/// Entry point for any user-space threads.
///
/// The ELF code is loaded prior to the start of `UserTask`.
/// `UserTask` will prepare the stacks and arguments, and switch to usermode.
pub struct UserTask {
    entry: Option<*const extern "C" fn()>,
    args: Option<Vec<CString>>,
    elf: Option<Vec<u8>>,
}

impl UserTask {
    const USER_STACK_START: Address<V> = Address::new(0x111900000);
    const USER_STACK_PAGES: usize = 4; // Too many???
    const USER_STACK_SIZE: usize = Self::USER_STACK_PAGES * Size4K::BYTES;

    /// Create a main thread.
    pub fn new_main(elf: Option<Vec<u8>>, args: Option<Vec<CString>>) -> Self {
        Self {
            entry: None,
            args,
            elf,
        }
    }

    /// Create a secondary thread
    pub fn new_companion(entry: *const extern "C" fn(), args: Option<Vec<CString>>) -> Self {
        Self {
            entry: Some(entry),
            args,
            elf: None,
        }
    }

    /// Spawn a new user process.
    pub fn spawn_user_process(
        elf: Vec<u8>,
        args: &[&str],
        affinity: Option<usize>,
    ) -> Arc<dyn Proc> {
        PROCESS_MANAGER.spawn(
            box UserTask::new_main(
                Some(elf),
                Some(args.iter().map(|s| CString::new(*s).unwrap()).collect()),
            ),
            affinity,
        )
    }

    fn setup_user_stack(page_table: &mut PageTable) -> Address {
        let tid = PROCESS_MANAGER.current_task().unwrap().id();
        let i = PROCESS_MANAGER
            .current_proc()
            .unwrap()
            .tasks()
            .lock_uninterruptible()
            .iter()
            .position(|t| *t == tid)
            .unwrap();
        // println!("User stack #{}", i);
        let user_stack_start = Self::USER_STACK_START + i * Self::USER_STACK_SIZE;
        for i in 0..Self::USER_STACK_PAGES {
            let page = Step::forward(Page::<Size4K>::new(user_stack_start), i);
            let frame = PHYSICAL_MEMORY.acquire::<Size4K>().unwrap();
            let _guard = KERNEL_MEMORY_MAPPER.with_kernel_address_space();
            page_table.map(page, frame, PageFlags::user_stack_flags(), &PHYSICAL_MEMORY);
        }
        user_stack_start + Self::USER_STACK_SIZE
    }
}

impl Runnable for UserTask {
    fn run(&mut self) -> ! {
        let proc = PROCESS_MANAGER.current_proc().unwrap();
        let first_thread = proc.tasks().lock().len() == 1;
        let entry = if first_thread {
            // First user thread of the process. Initialize the user space first.
            let initializer = UserProcessInitializer(proc.clone());
            initializer.initialize_user_space(self.elf.as_ref().unwrap())
        } else {
            // The process is spawning a new thread. The entrypoint is passed by the user program.
            unsafe { transmute(self.entry.unwrap()) }
        };
        let page_table = MMState::of(&*proc).get_page_table();
        // Setup user stack
        let mut stack_top = Self::setup_user_stack(page_table);
        // Prepare arguments
        let (arg0, arg1) = if first_thread {
            let args = self.args.as_ref().unwrap();
            let argc = args.len();
            let mut ptrs: Vec<*const u8> = Vec::with_capacity(argc);
            for arg in args {
                let buf = arg.to_bytes_with_nul();
                let ptr = stack_top - buf.len();
                unsafe { copy_nonoverlapping(buf.as_ptr(), ptr.as_mut_ptr(), buf.len()) };
                ptrs.push(ptr.as_ptr());
                stack_top = ptr;
            }
            for ptr in ptrs {
                stack_top = stack_top - size_of::<*const u8>();
                unsafe { stack_top.store(ptr) };
            }
            (argc as isize, stack_top.as_ptr::<*const u8>())
        } else {
            // TODO: Pass a context pointer
            (0, 0 as _)
        };
        // Enter usermode
        unsafe {
            <TargetArch as Arch>::Context::enter_usermode(entry, stack_top, page_table, arg0, arg1)
        }
    }
}

struct UserProcessInitializer(Arc<dyn Proc>);

impl UserProcessInitializer {
    fn initialize_user_space(&self, elf: &[u8]) -> extern "C" fn(isize, *const *const u8) {
        // log!("Initialze user space process");
        debug_assert_eq!(self.0.id(), PROCESS_MANAGER.current_proc().unwrap().id());
        // User page table
        let page_table = {
            let page_table = PageTable::alloc(&PHYSICAL_MEMORY);
            // Map kernel pages
            let kernel_memory = KERNEL_MEMORY_RANGE;
            let index = PageTable::<L4>::get_index(kernel_memory.start);
            debug_assert_eq!(index, PageTable::<L4>::get_index(kernel_memory.end - 1));
            page_table[index] = PageTable::get()[index].clone();
            self.set_page_table(unsafe { &mut *(page_table as *mut _) });
            PageTable::set(page_table);
            page_table
        };
        // log!("Load ELF");
        let entry = self.load_elf(page_table, elf);
        entry
    }

    fn load_elf(
        &self,
        page_table: &mut PageTable,
        elf_data: &[u8],
    ) -> extern "C" fn(isize, *const *const u8) {
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

    #[inline]
    fn set_page_table(&self, page_table: &'static mut PageTable) {
        MMState::of(&*self.0)
            .page_table
            .store(page_table, Ordering::SeqCst)
    }
}
