use core::ops::{Deref, DerefMut, Range};

use spin::Mutex;

use crate::utils::{address::*, page::*};

use super::page_table::{kernel::KernelPageTable, PageFlags};

pub struct KernelMemoryMapper {
    page_table: Mutex<Option<Frame>>,
}

impl KernelMemoryMapper {
    pub const fn new() -> Self {
        Self {
            page_table: Mutex::new(None),
        }
    }

    pub fn init(&self) {
        let page_table = KernelPageTable::get();
        *self.page_table.lock() = Some(Frame::new(page_table.into()))
    }

    fn with_kernel_page_table(&self) -> impl Drop + DerefMut + Deref<Target = KernelPageTable> {
        struct X(Frame);
        impl Drop for X {
            fn drop(&mut self) {
                // self.
            }
        }
        impl Deref for X {
            type Target = KernelPageTable;
            fn deref(&self) -> &Self::Target {
                unsafe { self.0.start().as_ref() }
            }
        }
        impl DerefMut for X {
            fn deref_mut(&mut self) -> &mut Self::Target {
                unsafe { self.0.start().as_mut() }
            }
        }
        X(self.page_table.lock().unwrap())
    }

    pub fn map_fixed<S: PageSize>(&self, page: Page<S>, frame: Frame<S>, flags: PageFlags) {
        let mut page_table = self.with_kernel_page_table();
        page_table.map(page, frame, flags);
    }
}

pub static KERNEL_MEMORY_MAPPER: KernelMemoryMapper = KernelMemoryMapper::new();
