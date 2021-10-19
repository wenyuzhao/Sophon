use super::KernelTask;
use crate::arch::*;
use crate::memory::kernel::KERNEL_MEMORY_MAPPER;
use crate::memory::physical::PHYSICAL_MEMORY;
use crate::task::Proc;
use crate::task::Task;
use core::iter::Step;
use interrupt::UninterruptibleMutex;
use memory::address::*;
use memory::page::*;
use memory::page_table::{PageFlags, PageFlagsExt, PageTable};

const USER_STACK_START: Address<V> = Address::new(0x111900000);
const USER_STACK_PAGES: usize = 4; // Too many???
const USER_STACK_SIZE: usize = USER_STACK_PAGES * Size4K::BYTES;

pub struct UserTask {
    entry: Option<*const extern "C" fn()>,
}

impl UserTask {
    pub fn new(entry: Option<*const extern "C" fn()>) -> Self {
        Self { entry }
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
}

impl KernelTask for UserTask {
    fn run(&mut self) -> ! {
        log!("User task start (kernel)");
        log!("Execute user program");
        let proc = Proc::current();
        if Proc::current().threads.lock().len() == 1 {
            let entry = proc.initialize_user_space();
            let page_table = proc.get_page_table();
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
