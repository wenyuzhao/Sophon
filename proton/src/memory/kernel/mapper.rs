use super::super::page_table::{kernel::PageTable, PageFlags};
use crate::{memory::kernel::KERNEL_HEAP_RANGE, utils::page::*};
use core::ops::{Deref, DerefMut};
use spin::Mutex;

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
        let page_table = PageTable::get();
        *self.page_table.lock() = Some(Frame::new(page_table.into()))
    }

    pub fn with_kernel_page_table(&self) -> impl Drop + DerefMut + Deref<Target = PageTable> {
        let page_table = unsafe {
            self.page_table
                .lock()
                .unwrap()
                .start()
                .as_mut::<PageTable>()
        };
        page_table.enable_temporarily()
    }

    pub fn acquire_physical_page<S: PageSize>(&self) -> Option<Frame<S>> {
        let _guard = self.with_kernel_page_table();
    }

    pub fn map_fixed<S: PageSize>(&self, page: Page<S>, frame: Frame<S>, flags: PageFlags) {
        debug_assert!(
            page.start() >= KERNEL_HEAP_RANGE.start && page.start() < KERNEL_HEAP_RANGE.end
        );
        let mut page_table = self.with_kernel_page_table();
        page_table.map(page, frame, flags);
    }
}

pub static KERNEL_MEMORY_MAPPER: KernelMemoryMapper = KernelMemoryMapper::new();
