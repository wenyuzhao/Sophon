use core::ops::{Deref, DerefMut, Range};

use crate::{memory::kernel::KERNEL_HEAP_RANGE, utils::page::*};
use spin::Mutex;

use super::page_table::{kernel::PageTable, PageFlags};

pub trait PhysicalPageResource: Sized {
    fn init(&mut self, frames: &'static [Range<Frame>]);
    fn acquire<S: PageSize>(&mut self) -> Option<Frame<S>>;
    fn release<S: PageSize>(&mut self, frame: Frame<S>);
}

mod buddy;
// mod monotone;

static PHYSICAL_PAGE_RESOURCE: Mutex<impl PhysicalPageResource> = Mutex::new(buddy::Buddy::new());

pub struct KernelMemoryMapper {
    page_table: Mutex<Option<Frame>>,
}

impl KernelMemoryMapper {
    pub const fn new() -> Self {
        Self {
            page_table: Mutex::new(None),
        }
    }

    pub fn init(&self, frames: &'static [Range<Frame>]) {
        PHYSICAL_PAGE_RESOURCE.lock().init(frames);
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
        PHYSICAL_PAGE_RESOURCE.lock().acquire()
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
