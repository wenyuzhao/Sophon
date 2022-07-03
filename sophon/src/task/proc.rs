use crate::kernel_tasks::user::UserTask;
use crate::memory::kernel::{KERNEL_MEMORY_MAPPER, KERNEL_MEMORY_RANGE};
use crate::memory::physical::PHYSICAL_MEMORY;
use crate::task::scheduler::{AbstractScheduler, SCHEDULER};
use crate::{kernel_tasks::KernelTask, task::Task};
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::{boxed::Box, collections::LinkedList, vec, vec::Vec};
use atomic::{Atomic, Ordering};
use core::iter::Step;
use core::ops::Range;
use core::ptr;
use core::sync::atomic::AtomicUsize;
use elf_rs::{Elf, ElfFile, ProgramType};
use interrupt::UninterruptibleMutex;
use ipc::scheme::{Resource, SchemeId};
use ipc::{ProcId, TaskId};
use memory::address::{Address, V};
use memory::page::{Page, PageSize, Size4K};
use memory::page_table::{PageFlags, PageFlagsExt, PageTable, L4};
use spin::Mutex;

static PROCS: Mutex<LinkedList<Arc<Proc>>> = Mutex::new(LinkedList::new());

pub struct Proc {
    pub id: ProcId,
    pub threads: Mutex<Vec<TaskId>>,
    page_table: Atomic<*mut PageTable>,
    pub resources: Mutex<BTreeMap<Resource, SchemeId>>,
    virtual_memory_highwater: Atomic<Address<V>>,
    user_elf: Option<Vec<u8>>,
}

unsafe impl Send for Proc {}
unsafe impl Sync for Proc {}

impl Proc {
    fn create(t: Box<dyn KernelTask>, user_elf: Option<Vec<u8>>) -> Arc<Proc> {
        // Assign an id
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let proc_id = ProcId(COUNTER.fetch_add(1, Ordering::SeqCst));
        // Allocate proc struct
        let proc = Arc::new(Proc {
            id: proc_id,
            threads: Mutex::new(vec![]),
            page_table: {
                // the initial page table is the kernel page table
                let _guard = KERNEL_MEMORY_MAPPER.with_kernel_address_space();
                Atomic::new(PageTable::get())
            },
            resources: Mutex::new(BTreeMap::new()),
            virtual_memory_highwater: Atomic::new(crate::memory::USER_SPACE_MEMORY_RANGE.start),
            user_elf,
        });
        // Create main thread
        let task = Task::create(proc.clone(), t);
        proc.threads.lock().push(task.id);
        // Add to list
        PROCS.lock_uninterruptible().push_back(proc.clone());
        // Spawn
        SCHEDULER.register_new_task(task);
        proc
    }

    fn load_elf(&self, page_table: &mut PageTable) -> extern "C" fn(isize, *const *const u8) {
        let elf_data: &[u8] = self.user_elf.as_ref().unwrap();
        let elf = Elf::from_bytes(elf_data).unwrap();
        if let Elf::Elf64(elf) = elf {
            log!("Parsed ELF file");
            let entry: extern "C" fn(isize, *const *const u8) =
                unsafe { ::core::mem::transmute(elf.elf_header().entry_point()) };
            log!("Entry: {:?}", entry as *mut ());
            let mut load_start = None;
            let mut load_end = None;
            let mut update_load_range = |start: Address, end: Address| match (load_start, load_end)
            {
                (None, None) => {
                    load_start = Some(start);
                    load_end = Some(end);
                }
                (Some(s), Some(e)) => {
                    if start < s {
                        load_start = Some(start)
                    }
                    if end > e {
                        load_end = Some(end)
                    }
                }
                _ => unreachable!(),
            };
            for p in elf
                .program_header_iter()
                .filter(|p| p.ph_type() == ProgramType::LOAD)
            {
                log!("{:?}", p);
                let start: Address = (p.vaddr() as usize).into();
                let end = start + (p.filesz() as usize);
                update_load_range(start, end);
            }
            if let Some(bss) = elf.lookup_section(".bss".as_bytes()) {
                log!("{:?}", bss);
                let start = Address::<V>::from(bss.addr() as usize);
                let end = start + bss.size() as usize;
                update_load_range(start, end);
            }
            log!(
                "vaddr: {:?} .. {:?}",
                load_start.unwrap(),
                load_end.unwrap()
            );
            let vaddr_start = Page::<Size4K>::align(load_start.unwrap());
            let vaddr_end = load_end.unwrap().align_up(Size4K::BYTES);
            let pages = (vaddr_end - vaddr_start) >> Page::<Size4K>::LOG_BYTES;
            let start_page = Page::<Size4K>::new(vaddr_start);
            for i in 0..pages {
                let page = Step::forward(start_page, i);
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
            // Copy data
            for p in elf
                .program_header_iter()
                .filter(|p| p.ph_type() == ProgramType::LOAD)
            {
                let start: Address = (p.vaddr() as usize).into();
                let bytes = p.filesz() as usize;
                let offset = p.offset() as usize;
                unsafe {
                    ptr::copy_nonoverlapping::<u8>(&elf_data[offset], start.as_mut_ptr(), bytes);
                }
            }
            if let Some(bss) = elf.lookup_section(".bss".as_bytes()) {
                let start = Address::<V>::from(bss.addr() as usize);
                let size = bss.size() as usize;
                unsafe {
                    ptr::write_bytes::<u8>(start.as_mut_ptr(), 0, size);
                }
            }
            memory::cache::flush_cache(vaddr_start..vaddr_end);
            entry
        } else {
            unimplemented!("elf32 is not supported")
        }
    }

    pub fn initialize_user_space(&self) -> extern "C" fn(isize, *const *const u8) {
        log!("Initialze user space process");
        // User page table
        let page_table = {
            let page_table = PageTable::alloc(&PHYSICAL_MEMORY);
            // Map kernel pages
            let kernel_memory = KERNEL_MEMORY_RANGE;
            let index = PageTable::<L4>::get_index(kernel_memory.start);
            debug_assert_eq!(index, PageTable::<L4>::get_index(kernel_memory.end - 1));
            page_table[index] = PageTable::get()[index].clone();
            Proc::current().set_page_table(unsafe { &mut *(page_table as *mut _) });
            page_table
        };
        log!("Load ELF");
        let entry = self.load_elf(page_table);
        self.set_page_table(page_table);
        entry
    }

    pub fn spawn(t: Box<dyn KernelTask>) -> Arc<Proc> {
        Self::create(t, None)
    }

    pub fn spawn_user(elf: Vec<u8>) -> Arc<Proc> {
        Self::create(box UserTask::new(None), Some(elf))
    }

    #[inline]
    pub fn get_page_table(&self) -> &'static mut PageTable {
        unsafe { &mut *self.page_table.load(Ordering::SeqCst) }
    }

    #[inline]
    pub fn set_page_table(&self, page_table: &'static mut PageTable) {
        self.page_table.store(page_table, Ordering::SeqCst)
    }

    #[inline]
    pub fn current() -> Arc<Proc> {
        Task::current().proc.clone()
    }

    pub fn spawn_task(self: Arc<Self>, f: *const extern "C" fn()) -> Arc<Task> {
        let task = Task::create(self.clone(), box UserTask::new(Some(f)));
        self.threads.lock_uninterruptible().push(task.id);
        SCHEDULER.register_new_task(task)
    }

    pub fn sbrk(&self, num_pages: usize) -> Option<Range<Page<Size4K>>> {
        let result =
            self.virtual_memory_highwater
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |old| {
                    let old_aligned = old.align_up(Size4K::BYTES);
                    Some(old_aligned + (num_pages << Size4K::LOG_BYTES))
                });
        log!("sbrk: {:?} {:?}", self.id, result);
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

    pub fn exit(&self) {
        // Release file handles
        for (_resourse, _scheme_id) in self.resources.lock().iter() {
            // TODO: close `resourse`
        }
        // Release memory
        let _guard = KERNEL_MEMORY_MAPPER.with_kernel_address_space();
        crate::memory::utils::release_user_page_table(self.get_page_table());
    }
}
