use core::{any::Any, sync::atomic::AtomicUsize};

use alloc::{borrow::ToOwned, boxed::Box, collections::BTreeMap, sync::Arc, vec, vec::Vec};
use atomic::Ordering;
use kernel_module::SERVICE;
use proc::{Proc, ProcId, Runnable, Task, TaskId};
use spin::{Lazy, Mutex};
use sync::Monitor;

use crate::locks::{RawCondvar, RawMutex};

static PROCS: Mutex<BTreeMap<ProcId, Arc<Process>>> = Mutex::new(BTreeMap::new());

pub struct Process {
    pub id: ProcId,
    pub threads: Mutex<Vec<TaskId>>,
    pub live: Lazy<Monitor<bool>>,
    pub fs: Box<dyn Any>,
    pub mm: Box<dyn Any>,
    pub locks: Mutex<Vec<*mut RawMutex>>,
    pub cvars: Mutex<Vec<*mut RawCondvar>>,
}

unsafe impl Send for Process {}
unsafe impl Sync for Process {}

fn create_task(
    proc: Arc<dyn Proc>,
    runnable: Box<dyn Runnable>,
    context: Box<dyn Any>,
) -> Arc<Task> {
    static TASK_ID_COUNT: AtomicUsize = AtomicUsize::new(0);
    let id = TaskId(TASK_ID_COUNT.fetch_add(1, Ordering::SeqCst));
    let task = Arc::new(Task {
        id,
        context,
        proc: Arc::downgrade(&proc),
        live: Lazy::new(|| Monitor::new(true)),
        sched: SERVICE.scheduler().new_state(),
        runnable,
    });
    crate::TASKS.lock().insert(task.id, task.clone());
    task
}

fn current_task() -> Option<Arc<Task>> {
    let id = SERVICE.scheduler().get_current_task_id()?;
    crate::TASKS.lock().get(&id).cloned()
}

impl Process {
    pub fn create(t: Box<dyn Runnable>, mm: Box<dyn Any>) -> Arc<Self> {
        let _guard = interrupt::uninterruptible();
        // Assign an id
        static COUNTER: AtomicUsize = AtomicUsize::new(1);
        let proc_id = ProcId(COUNTER.fetch_add(1, Ordering::SeqCst));
        // Allocate proc struct
        let vfs_state = SERVICE.vfs().register_process(proc_id, "".to_owned());
        let proc = Arc::new(Self {
            id: proc_id,
            threads: Mutex::new(vec![]),
            mm,
            live: Lazy::new(|| Monitor::new(true)),
            fs: vfs_state,
            locks: Default::default(),
            cvars: Default::default(),
        });
        // Create main thread
        let task = create_task(proc.clone(), t, SERVICE.create_task_context());
        proc.threads.lock().push(task.id);
        // Add to list
        PROCS.lock().insert(proc.id, proc.clone());
        // Spawn
        SERVICE.scheduler().register_new_task(task.id);
        proc
    }

    #[inline(always)]
    pub const fn as_dyn(self: Arc<Process>) -> Arc<dyn Proc> {
        self
    }

    #[inline(always)]
    pub fn by_id(id: ProcId) -> Option<Arc<Self>> {
        let _guard = interrupt::uninterruptible();
        PROCS.lock().get(&id).cloned()
    }

    #[inline(always)]
    pub fn current() -> Option<Arc<Self>> {
        let _guard = interrupt::uninterruptible();
        let proc = current_task().map(|t| t.proc.upgrade().unwrap())?;
        let ptr = Arc::into_raw(proc).cast::<Self>();
        Some(unsafe { Arc::from_raw(ptr) })
    }
}

impl Proc for Process {
    fn id(&self) -> ProcId {
        self.id
    }
    fn fs(&self) -> &dyn Any {
        self.fs.as_ref()
    }
    fn mm(&self) -> &dyn Any {
        self.mm.as_ref()
    }
    fn tasks(&self) -> &Mutex<Vec<TaskId>> {
        &self.threads
    }
    fn spawn_task(self: Arc<Self>, task: Box<dyn Runnable>) -> Arc<Task> {
        let _guard = interrupt::uninterruptible();
        let task = create_task(self.clone(), task, SERVICE.create_task_context());
        self.threads.lock().push(task.id);
        SERVICE.scheduler().register_new_task(task.id);
        debug_assert_eq!(Arc::strong_count(&task), 2);
        task
    }
    fn exit(&self) {
        let _guard = interrupt::uninterruptible();
        // Release file handles
        SERVICE.vfs().deregister_process(self.id);
        // Release memory
        // - Note: this is done in the MMState destructor
        // Mark as dead
        {
            let mut live = self.live.lock();
            *live = false;
            self.live.notify_all();
        }
        // Remove from scheduler
        let threads = self.threads.lock();
        for t in &*threads {
            crate::TASKS.lock().remove(t).unwrap();
            SERVICE.scheduler().remove_task(*t)
        }
        // Remove from procs
        PROCS.lock().remove(&self.id);
    }
    fn wait_for_completion(&self) {
        let mut live = self.live.lock();
        while *live {
            live = self.live.wait(live);
        }
    }
}
impl PartialEq for Process {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Process {}
