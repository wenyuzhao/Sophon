use crate::memory::kernel::KERNEL_HEAP_RANGE;
use crate::memory::physical::PHYSICAL_MEMORY;
use atomic::Ordering;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::AtomicPtr;
use memory::address::{Address, P, V};
use memory::page::*;
use memory::page_table::{PageFlagSet, PageTable};

pub struct KernelMemoryMapper {
    page_table: AtomicPtr<PageTable>,
}

unsafe impl Sync for KernelMemoryMapper {}

impl KernelMemoryMapper {
    pub const fn new() -> Self {
        Self {
            page_table: AtomicPtr::new(core::ptr::null_mut()),
        }
    }

    pub fn init(&self) {
        let page_table = PageTable::get();
        self.page_table.store(page_table, Ordering::SeqCst);
    }

    pub fn get_kernel_page_table(&self) -> *const PageTable {
        self.page_table.load(Ordering::SeqCst)
    }

    /// Temporarily enable kernel address space.
    pub fn with_kernel_address_space(&self) -> impl Drop + DerefMut + Deref<Target = PageTable> {
        debug_assert!(!self.page_table.load(Ordering::SeqCst).is_null());
        let page_table = unsafe { &mut *self.page_table.load(Ordering::SeqCst) };
        page_table.enable_temporarily()
    }

    /// Map a virtual page to a physical page
    pub fn map<S: PageSize>(&self, page: Page<S>, frame: Frame<S>, flags: PageFlagSet) {
        debug_assert!(
            page.start() >= KERNEL_HEAP_RANGE.start && page.start() < KERNEL_HEAP_RANGE.end
        );
        let mut page_table = self.with_kernel_address_space();
        page_table.map(page, frame, flags, &PHYSICAL_MEMORY);
    }

    /// Unmap a virtual page (does not release the physical page)
    pub fn unmap<S: PageSize>(&self, page: Page<S>) {
        debug_assert!(
            page.start() >= KERNEL_HEAP_RANGE.start && page.start() < KERNEL_HEAP_RANGE.end
        );
        let mut page_table = self.with_kernel_address_space();
        page_table.unmap(page, &PHYSICAL_MEMORY);
    }

    /// Unmap a virtual page (does not release the physical page)
    pub fn translate(&self, v: Address<V>) -> Option<Address<P>> {
        let mut page_table = self.with_kernel_address_space();
        page_table.translate(v)
    }
}

pub static KERNEL_MEMORY_MAPPER: KernelMemoryMapper = KernelMemoryMapper::new();
