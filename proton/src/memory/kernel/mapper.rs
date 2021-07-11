use super::super::page_table::{kernel::KernelPageTable, PageFlags};
use crate::{
    arch::{Arch, TargetArch},
    memory::kernel::KERNEL_HEAP_RANGE,
    utils::page::*,
};
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
        let page_table = KernelPageTable::get();
        *self.page_table.lock() = Some(Frame::new(page_table.into()))
    }

    fn with_kernel_page_table(&self) -> impl Drop + DerefMut + Deref<Target = KernelPageTable> {
        struct PageTables {
            old: Frame,
            new: Frame,
        }
        impl Drop for PageTables {
            fn drop(&mut self) {
                if self.old != self.new {
                    TargetArch::set_current_page_table(self.old);
                }
            }
        }
        impl Deref for PageTables {
            type Target = KernelPageTable;
            fn deref(&self) -> &Self::Target {
                unsafe { self.new.start().as_ref() }
            }
        }
        impl DerefMut for PageTables {
            fn deref_mut(&mut self) -> &mut Self::Target {
                unsafe { self.new.start().as_mut() }
            }
        }
        let x = PageTables {
            old: TargetArch::get_current_page_table(),
            new: self.page_table.lock().unwrap(),
        };
        if x.old != x.new {
            TargetArch::set_current_page_table(x.new);
        }
        x
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
