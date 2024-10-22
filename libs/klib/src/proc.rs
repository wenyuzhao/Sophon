use core::{
    any::Any,
    sync::atomic::{AtomicBool, AtomicPtr, Ordering},
};

use alloc::{boxed::Box, vec::Vec};
use atomic::Atomic;
use memory::{
    address::{Address, V},
    page_table::{PageTable, L4},
};
use spin::Mutex;

use crate::task::TaskId;

#[derive(Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Clone, Copy)]
pub struct PID(pub usize);

impl PID {
    pub const NULL: Self = Self(0);
}

pub struct Process {
    pub id: PID,
    pub threads: Mutex<Vec<TaskId>>,
    pub mem: Box<MemSpace>,
    pub fs: Box<dyn Any>,
}

pub struct MemSpace {
    pub page_table: AtomicPtr<PageTable<L4>>,
    pub has_user_page_table: AtomicBool,
    pub highwater: Atomic<Address<V>>,
}

impl MemSpace {
    pub fn has_user_page_table(&self) -> bool {
        self.has_user_page_table.load(Ordering::SeqCst)
    }

    pub fn get_page_table(&self) -> &'static mut PageTable {
        unsafe { &mut *self.page_table.load(Ordering::SeqCst) }
    }
}
