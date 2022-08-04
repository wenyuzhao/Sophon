use super::runnable::UserTask;
use super::{runnable::Runnable, task::Task};
use super::{ProcId, TaskId};
use crate::memory::kernel::{KERNEL_MEMORY_MAPPER, KERNEL_MEMORY_RANGE};
use crate::memory::physical::PHYSICAL_MEMORY;
use crate::scheduler::monitor::SysMonitor;
use crate::scheduler::{AbstractScheduler, SCHEDULER};
use alloc::ffi::CString;
use alloc::sync::Arc;
use alloc::{boxed::Box, collections::LinkedList, vec, vec::Vec};
use atomic::{Atomic, Ordering};
use core::iter::Step;
use core::ops::Range;
use core::sync::atomic::AtomicUsize;
use interrupt::UninterruptibleMutex;
use memory::address::{Address, V};
use memory::page::{Page, PageSize, Size4K};
use memory::page_table::{PageFlags, PageFlagsExt, PageTable, L4};
use mutex::AbstractMonitor;
use spin::Mutex;
use vfs::VFSRequest;

static PROCS: Mutex<LinkedList<Arc<Proc>>> = Mutex::new(LinkedList::new());

pub struct Proc {
    pub id: ProcId,
    pub threads: Mutex<Vec<TaskId>>,
    page_table: Atomic<*mut PageTable>,
    virtual_memory_highwater: Atomic<Address<V>>,
    user_elf: Option<Vec<u8>>,
    pub monitor: Arc<SysMonitor>,
}

unsafe impl Send for Proc {}
unsafe impl Sync for Proc {}

impl Proc {
    fn create(t: Box<dyn Runnable>, user_elf: Option<Vec<u8>>) -> Arc<Proc> {
        // Assign an id
        static COUNTER: AtomicUsize = AtomicUsize::new(1);
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
            virtual_memory_highwater: Atomic::new(crate::memory::USER_SPACE_MEMORY_RANGE.start),
            user_elf,
            monitor: SysMonitor::new(),
        });
        // Create main thread
        let task = Task::create(proc.clone(), t);
        proc.threads.lock().push(task.id);
        // Add to list
        PROCS.lock_uninterruptible().push_back(proc.clone());
        // Notify VFS
        crate::modules::module_call(
            "vfs",
            true,
            &VFSRequest::ProcStart {
                proc: proc.id,
                parent: Proc::current_opt().map(|p| p.id).unwrap_or(ProcId::KERNEL),
                cwd: "",
            },
        );
        // Spawn
        SCHEDULER.register_new_task(task);
        proc
    }

    fn load_elf(&self, page_table: &mut PageTable) -> extern "C" fn(isize, *const *const u8) {
        let elf_data: &[u8] = self.user_elf.as_ref().unwrap();
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
        // log!("Entry: {:?}", entry);
        unsafe { core::mem::transmute(entry) }
    }

    pub fn initialize_user_space(&self) -> extern "C" fn(isize, *const *const u8) {
        // log!("Initialze user space process");
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
        // log!("Load ELF");
        let entry = self.load_elf(page_table);
        self.set_page_table(page_table);
        entry
    }

    pub fn spawn(t: Box<dyn Runnable>) -> Arc<Proc> {
        Self::create(t, None)
    }

    pub fn spawn_user(elf: Vec<u8>, args: &[&str]) -> Arc<Proc> {
        Self::create(
            box UserTask::new(
                None,
                Some(args.iter().map(|s| CString::new(*s).unwrap()).collect()),
            ),
            Some(elf),
        )
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

    #[inline]
    pub fn current_opt() -> Option<Arc<Proc>> {
        Task::current_opt().map(|t| t.proc.clone())
    }

    pub fn spawn_task(self: Arc<Self>, f: *const extern "C" fn()) -> Arc<Task> {
        let task = Task::create(self.clone(), box UserTask::new(Some(f), None));
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

    pub fn exit(&self) {
        // Release file handles
        crate::modules::module_call("vfs", true, &VFSRequest::ProcExit(self.id));
        // Release memory
        let _guard = KERNEL_MEMORY_MAPPER.with_kernel_address_space();
        crate::memory::utils::release_user_page_table(self.get_page_table());
        // Remove from scheduler
        let threads = self.threads.lock();
        for t in &*threads {
            SCHEDULER.remove_task(*t)
        }
        self.monitor.notify();
    }
}
