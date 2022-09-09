use super::runnable::UserTask;
use crate::arch::ArchContext;
use crate::memory::kernel::{KERNEL_MEMORY_MAPPER, KERNEL_MEMORY_RANGE};
use crate::memory::physical::PHYSICAL_MEMORY;
use crate::modules::PROCESS_MANAGER;
use alloc::ffi::CString;
use alloc::sync::Arc;
use alloc::{boxed::Box, vec::Vec};
use atomic::{Atomic, Ordering};
use core::any::Any;
use core::iter::Step;
use core::ops::{Deref, Range};
use memory::address::{Address, V};
use memory::page::{Page, PageSize, Size4K};
use memory::page_table::{PageFlags, PageTable, L4};
use proc::{Proc, Task};

pub struct MMState {
    page_table: Atomic<*mut PageTable>,
    virtual_memory_highwater: Atomic<Address<V>>,
}

impl MMState {
    pub fn new() -> Box<dyn Any> {
        let x = Self {
            page_table: {
                // the initial page table is the kernel page table
                let _guard = KERNEL_MEMORY_MAPPER.with_kernel_address_space();
                Atomic::new(PageTable::get())
            },
            virtual_memory_highwater: Atomic::new(crate::memory::USER_SPACE_MEMORY_RANGE.start),
        };
        box x
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

pub struct ProcUtils;

impl ProcUtils {
    pub fn spawn_user(elf: Vec<u8>, args: &[&str]) -> Arc<dyn Proc> {
        PROCESS_MANAGER.spawn(
            box UserTask::new_main(
                Some(elf),
                Some(args.iter().map(|s| CString::new(*s).unwrap()).collect()),
            ),
            MMState::new(),
        )
    }
}

pub trait ProcExt {
    fn initialize_user_space(&self, elf: &[u8]) -> extern "C" fn(isize, *const *const u8);
    fn get_mm_state(&self) -> &MMState;
    fn get_page_table(&self) -> &'static mut PageTable;
    fn set_page_table(&self, page_table: &'static mut PageTable);
    fn load_elf(
        &self,
        page_table: &mut PageTable,
        elf_data: &[u8],
    ) -> extern "C" fn(isize, *const *const u8);
    fn sbrk(&self, num_pages: usize) -> Option<Range<Page<Size4K>>>;
}

impl ProcExt for Arc<dyn Proc> {
    fn initialize_user_space(&self, elf: &[u8]) -> extern "C" fn(isize, *const *const u8) {
        // log!("Initialze user space process");
        debug_assert_eq!(self.id(), PROCESS_MANAGER.current_proc().unwrap().id());
        // User page table
        let page_table = {
            let page_table = PageTable::alloc(&PHYSICAL_MEMORY);
            // Map kernel pages
            let kernel_memory = KERNEL_MEMORY_RANGE;
            let index = PageTable::<L4>::get_index(kernel_memory.start);
            debug_assert_eq!(index, PageTable::<L4>::get_index(kernel_memory.end - 1));
            page_table[index] = PageTable::get()[index].clone();
            self.set_page_table(unsafe { &mut *(page_table as *mut _) });
            PageTable::set(page_table);
            page_table
        };
        // log!("Load ELF");
        let entry = self.load_elf(page_table, elf);
        entry
    }

    fn load_elf(
        &self,
        page_table: &mut PageTable,
        elf_data: &[u8],
    ) -> extern "C" fn(isize, *const *const u8) {
        let base = Address::<V>::from(0x200000);
        let entry = elf_loader::ELFLoader::load(elf_data, &mut |pages| {
            let start_page = Page::new(base);
            let num_pages = Page::steps_between(&pages.start, &pages.end).unwrap();
            for (i, _) in pages.enumerate() {
                let page = Page::<Size4K>::forward(start_page, i);
                let frame = PHYSICAL_MEMORY.acquire::<Size4K>().unwrap();
                let _kernel_page_table = KERNEL_MEMORY_MAPPER.with_kernel_address_space();
                page_table.map(
                    page,
                    frame,
                    PageFlags::user_code_flags_4k(),
                    &PHYSICAL_MEMORY,
                );
            }
            PageTable::set(page_table);
            start_page..Page::<Size4K>::forward(start_page, num_pages)
        })
        .unwrap();
        // log!("Entry: {:?}", entry.entry);
        unsafe { core::mem::transmute(entry.entry) }
    }

    #[inline]
    fn get_mm_state(&self) -> &MMState {
        self.mm().downcast_ref().unwrap()
    }

    #[inline]
    fn get_page_table(&self) -> &'static mut PageTable {
        unsafe { &mut *self.get_mm_state().page_table.load(Ordering::SeqCst) }
    }

    #[inline]
    fn set_page_table(&self, page_table: &'static mut PageTable) {
        self.get_mm_state()
            .page_table
            .store(page_table, Ordering::SeqCst)
    }
    fn sbrk(&self, num_pages: usize) -> Option<Range<Page<Size4K>>> {
        let result = self.get_mm_state().virtual_memory_highwater.fetch_update(
            Ordering::SeqCst,
            Ordering::SeqCst,
            |old| {
                let old_aligned = old.align_up(Size4K::BYTES);
                Some(old_aligned + (num_pages << Size4K::LOG_BYTES))
            },
        );
        // log!("sbrk: {:?} {:?}", self.id, result);
        match result {
            Ok(a) => {
                let old_top = a;
                let start = Page::new(a.align_up(Size4K::BYTES));
                let end = Page::forward(start, num_pages);
                debug_assert_eq!(old_top, start.start());
                // Map old_top .. end
                {
                    let page_table = self.get_page_table();
                    let _guard = KERNEL_MEMORY_MAPPER.with_kernel_address_space();
                    for page in start..end {
                        let frame = PHYSICAL_MEMORY.acquire().unwrap();
                        page_table.map(
                            page,
                            frame,
                            PageFlags::user_data_flags_4k(),
                            &PHYSICAL_MEMORY,
                        );
                    }
                }
                Some(start..end)
            }
            Err(_e) => return None,
        }
    }
}

pub trait TaskExt {
    fn get_context<C: ArchContext>(&self) -> &C;
}

impl TaskExt for Arc<dyn Task> {
    fn get_context<C: ArchContext>(&self) -> &C {
        unsafe { self.context().downcast_ref_unchecked() }
    }
}
