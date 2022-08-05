use crate::arch::*;
use crate::memory::kernel::KERNEL_MEMORY_MAPPER;
use crate::memory::physical::PHYSICAL_MEMORY;
use crate::task::Proc;
use crate::task::Task;
use alloc::ffi::CString;
use alloc::vec::Vec;
use core::arch::asm;
use core::iter::Step;
use core::mem::size_of;
use core::mem::transmute;
use core::ptr::copy_nonoverlapping;
use interrupt::UninterruptibleMutex;
use memory::address::*;
use memory::page::*;
use memory::page_table::{PageFlags, PageTable};

/// Holds the execution code for a kernel task.
///
/// Unless jumping to the user mode, the program will remain in the kernel-space.
pub trait Runnable {
    fn run(&mut self) -> !;
}

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
}

impl UserTask {
    const USER_STACK_START: Address<V> = Address::new(0x111900000);
    const USER_STACK_PAGES: usize = 4; // Too many???
    const USER_STACK_SIZE: usize = Self::USER_STACK_PAGES * Size4K::BYTES;

    pub fn new(entry: Option<*const extern "C" fn()>, args: Option<Vec<CString>>) -> Self {
        Self { entry, args }
    }

    fn setup_user_stack(page_table: &mut PageTable) -> Address {
        let tid = Task::current().id;
        let i = Proc::current()
            .threads
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
        let proc = Proc::current();
        let first_thread = proc.threads.lock().len() == 1;
        let entry = if first_thread {
            // First user thread of the process. Initialize the user space first.
            proc.initialize_user_space()
        } else {
            // The process is spawning a new thread. The entrypoint is passed by the user program.
            unsafe { transmute(self.entry.unwrap()) }
        };
        let page_table = proc.get_page_table();
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
