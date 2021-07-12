mod physical_page_resource;

use self::physical_page_resource::PHYSICAL_PAGE_RESOURCE;
use super::kernel::KERNEL_MEMORY_MAPPER;
use crate::utils::page::*;
use core::ops::Range;

pub struct PhysicalMemory {
    _private: (),
}

impl PhysicalMemory {
    pub const fn new() -> Self {
        Self { _private: () }
    }

    pub fn init(&self, frames: &'static [Range<Frame>]) {
        PHYSICAL_PAGE_RESOURCE.lock().init(frames);
        KERNEL_MEMORY_MAPPER.init();
    }

    pub fn acquire<S: PageSize>(&self) -> Option<Frame<S>> {
        let _guard = KERNEL_MEMORY_MAPPER.with_kernel_page_table();
        PHYSICAL_PAGE_RESOURCE.lock().acquire()
    }

    pub fn release<S: PageSize>(&self, frame: Frame<S>) {
        let _guard = KERNEL_MEMORY_MAPPER.with_kernel_page_table();
        PHYSICAL_PAGE_RESOURCE.lock().release(frame)
    }
}

pub static PHYSICAL_MEMORY: PhysicalMemory = PhysicalMemory::new();
