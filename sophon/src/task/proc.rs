use crate::memory::kernel::KERNEL_MEMORY_MAPPER;
use crate::memory::physical::PHYSICAL_MEMORY;
use crate::task::scheduler::{AbstractScheduler, SCHEDULER};
use crate::{kernel_tasks::KernelTask, task::Task, utils::unint_lock::UnintMutex};
use alloc::collections::BTreeMap;
use alloc::{boxed::Box, collections::LinkedList, vec, vec::Vec};
use atomic::{Atomic, Ordering};
use core::iter::Step;
use core::ops::Range;
use core::sync::atomic::AtomicUsize;
use ipc::scheme::{Resource, SchemeId};
use ipc::{ProcId, TaskId};
use memory::address::{Address, V};
use memory::page::{Page, PageSize, Size4K};
use memory::page_table::{PageFlags, PageFlagsExt, PageTable};

static PROCS: UnintMutex<LinkedList<Box<Proc>>> = UnintMutex::new(LinkedList::new());

pub struct Proc {
    pub id: ProcId,
    threads: Vec<TaskId>,
    page_table: Atomic<*mut PageTable>,
    pub resources: UnintMutex<BTreeMap<Resource, SchemeId>>,
    virtual_memory_highwater: Atomic<Address<V>>,
}

unsafe impl Send for Proc {}

impl Proc {
    pub fn spawn(t: Box<dyn KernelTask>) -> &'static Proc {
        // Assign an id
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let proc_id = ProcId(COUNTER.fetch_add(1, Ordering::SeqCst));
        // Allocate proc struct
        let mut proc = box Proc {
            id: proc_id,
            threads: vec![],
            page_table: {
                // the initial page table is the kernel page table
                let _guard = KERNEL_MEMORY_MAPPER.with_kernel_address_space();
                Atomic::new(PageTable::get())
            },
            resources: UnintMutex::new(BTreeMap::new()),
            virtual_memory_highwater: Atomic::new(crate::memory::USER_SPACE_MEMORY_RANGE.start),
        };
        let proc_mut = unsafe { &mut *(proc.as_mut() as *mut Proc) };
        // Create main thread
        let task = Task::create(unsafe { &mut *(proc.as_mut() as *mut Proc) }, t);
        proc.threads.push(task.id);
        // Add to list
        PROCS.lock().push_back(proc);
        // Spawn
        SCHEDULER.register_new_task(task);
        proc_mut
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
    pub fn current() -> &'static Proc {
        Task::current().proc
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
}
