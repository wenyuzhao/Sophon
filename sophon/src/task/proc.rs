use crate::memory::kernel::KERNEL_MEMORY_MAPPER;
use crate::task::scheduler::{AbstractScheduler, SCHEDULER};
use crate::{kernel_tasks::KernelTask, task::Task, utils::unint_lock::UnintMutex};
use alloc::{boxed::Box, collections::LinkedList, vec, vec::Vec};
use atomic::Ordering;
use core::sync::atomic::AtomicUsize;
use ipc::{ProcId, TaskId};
use memory::page_table::PageTable;

static PROCS: UnintMutex<LinkedList<Box<Proc>>> = UnintMutex::new(LinkedList::new());

pub struct Proc {
    id: ProcId,
    threads: Vec<TaskId>,
    pub page_table: &'static mut PageTable,
}

impl Proc {
    pub fn spawn(t: Box<dyn KernelTask>) -> &'static mut Proc {
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
                PageTable::get()
            },
        };
        let proc_mut = unsafe { &mut *(proc.as_mut() as *mut Proc) };
        let task = Task::create(unsafe { &mut *(proc.as_mut() as *mut Proc) }, t);
        proc.threads.push(task.id());
        // Add to list
        PROCS.lock().push_back(proc);
        // Spawn
        SCHEDULER.register_new_task(task);
        proc_mut
    }

    #[inline]
    pub fn id(&self) -> ProcId {
        self.id
    }

    #[inline]
    pub fn page_table(&self) -> &'static mut PageTable {
        unsafe { &mut *(self.page_table as *const PageTable as *mut PageTable) }
    }

    #[inline]
    pub fn current() -> &'static mut Proc {
        Task::current().unwrap().proc
    }
}
