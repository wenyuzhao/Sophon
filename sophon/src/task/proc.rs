use crate::memory::kernel::KERNEL_MEMORY_MAPPER;
use crate::task::scheduler::{AbstractScheduler, SCHEDULER};
use crate::{kernel_tasks::KernelTask, task::Task, utils::unint_lock::UnintMutex};
use alloc::{boxed::Box, collections::LinkedList, vec, vec::Vec};
use atomic::{Atomic, Ordering};
use core::sync::atomic::AtomicUsize;
use ipc::{ProcId, TaskId};
use memory::page_table::PageTable;

static PROCS: UnintMutex<LinkedList<Box<Proc>>> = UnintMutex::new(LinkedList::new());

pub struct Proc {
    pub id: ProcId,
    threads: Vec<TaskId>,
    page_table: Atomic<*mut PageTable>,
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
                let _guard = KERNEL_MEMORY_MAPPER.with_kernel_page_table();
                Atomic::new(PageTable::get())
            },
        };
        let proc_mut = unsafe { &mut *(proc.as_mut() as *mut Proc) };
        // Create main thread
        let task = Task::create(unsafe { &mut *(proc.as_mut() as *mut Proc) }, t);
        proc.threads.push(task.id());
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
        Task::current().unwrap().proc
    }
}
