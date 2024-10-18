use crate::memory::kernel::KERNEL_MEMORY_MAPPER;
use alloc::boxed::Box;
use atomic::{Atomic, Ordering};
use core::any::Any;
use core::ops::Deref;
use core::sync::atomic::AtomicPtr;
use memory::address::{Address, V};
use memory::page_table::PageTable;
use proc::Proc;

pub struct MMState {
    pub page_table: AtomicPtr<PageTable>,
    pub virtual_memory_highwater: Atomic<Address<V>>,
}

impl MMState {
    pub fn new() -> Box<dyn Any> {
        let x = Self {
            page_table: {
                // the initial page table is the kernel page table
                let _guard = KERNEL_MEMORY_MAPPER.with_kernel_address_space();
                AtomicPtr::new(PageTable::get())
            },
            virtual_memory_highwater: Atomic::new(crate::memory::USER_SPACE_MEMORY_RANGE.start),
        };
        Box::new(x)
    }

    pub fn get_page_table(&self) -> &'static mut PageTable {
        unsafe { &mut *self.page_table.load(Ordering::SeqCst) }
    }

    pub fn of(proc: &dyn Proc) -> &Self {
        proc.mm().downcast_ref().unwrap()
    }
}

impl Drop for MMState {
    fn drop(&mut self) {
        let guard = KERNEL_MEMORY_MAPPER.with_kernel_address_space();
        let user_page_table = unsafe { &mut *self.page_table.load(Ordering::SeqCst) };
        if guard.deref() as *const PageTable != user_page_table as *const PageTable {
            crate::memory::utils::release_user_page_table(user_page_table);
        }
    }
}
