use core::{any::Any, sync::atomic::AtomicUsize};

use alloc::{
    borrow::ToOwned,
    boxed::Box,
    collections::BTreeMap,
    sync::{Arc, Weak},
    vec,
    vec::Vec,
};
use atomic::Ordering;
use kernel_module::SERVICE;
use proc::{Proc, ProcId, Runnable, TaskId};
use spin::{Lazy, Mutex};
use sync::Monitor;

static PROCS: Mutex<BTreeMap<ProcId, Arc<Process>>> = Mutex::new(BTreeMap::new());
static TASKS: Mutex<BTreeMap<TaskId, Arc<Task>>> = Mutex::new(BTreeMap::new());

pub struct Process {
    pub id: ProcId,
    pub threads: Mutex<Vec<TaskId>>,
    pub live: Lazy<Monitor<bool>>,
    pub fs: Box<dyn Any>,
    pub mm: Box<dyn Any>,
    pub pm: Box<dyn Any>,
}

unsafe impl Send for Process {}
unsafe impl Sync for Process {}

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
            pm: SERVICE.process_manager().new_state(),
        });
        // Create main thread
        let task = Task::create(proc.clone(), t, SERVICE.create_task_context());
        proc.threads.lock().push(task.id);
        // Add to list
        PROCS.lock().insert(proc.id, proc.clone());
        // Spawn
        TASKS.lock().insert(task.id, task.clone());
        SERVICE.scheduler().register_new_task(task.id);
        proc
    }

    pub fn as_proc(self: Arc<Process>) -> Arc<dyn Proc> {
        self
    }

    pub fn by_id(id: ProcId) -> Option<Arc<Self>> {
        let _guard = interrupt::uninterruptible();
        PROCS.lock().get(&id).cloned()
    }

    pub fn current() -> Option<Arc<Self>> {
        let _guard = interrupt::uninterruptible();
        let proc = Task::current().map(|t| t.proc.upgrade().unwrap())?;
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
    fn spawn_task(self: Arc<Self>, task: Box<dyn Runnable>) -> Arc<dyn proc::Task> {
        let _guard = interrupt::uninterruptible();
        let task = Task::create(self.clone(), task, SERVICE.create_task_context());
        self.threads.lock().push(task.id);
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

pub struct Task {
    pub id: TaskId,
    pub context: Box<dyn Any>,
    proc: Weak<dyn Proc>,
    pub live: Lazy<Monitor<bool>>,
    pub sched: Box<dyn Any>,
    runnable: Box<dyn Runnable>,
}

impl proc::Task for Task {
    fn id(&self) -> TaskId {
        self.id
    }

    fn context(&self) -> &dyn Any {
        self.context.as_ref()
    }

    fn proc(&self) -> Arc<dyn Proc> {
        self.proc.upgrade().unwrap()
    }

    fn sched(&self) -> &dyn Any {
        self.sched.as_ref()
    }

    fn state(&self) -> &Monitor<bool> {
        &self.live
    }

    fn runnable(&self) -> &dyn Runnable {
        &*self.runnable
    }
}

impl Task {
    pub fn create(
        proc: Arc<dyn Proc>,
        runnable: Box<dyn Runnable>,
        context: Box<dyn Any>,
    ) -> Arc<Self> {
        static TASK_ID_COUNT: AtomicUsize = AtomicUsize::new(0);
        let id = TaskId(TASK_ID_COUNT.fetch_add(1, Ordering::SeqCst));
        Arc::new(Task {
            id,
            context,
            proc: Arc::downgrade(&proc),
            live: Lazy::new(|| Monitor::new(true)),
            sched: SERVICE.scheduler().new_state(),
            runnable,
        })
    }

    pub fn by_id(id: TaskId) -> Option<Arc<Self>> {
        TASKS.lock().get(&id).cloned()
    }

    pub fn as_dyn(self: Arc<Self>) -> Arc<dyn proc::Task> {
        self
    }

    pub fn current() -> Option<Arc<Self>> {
        Self::by_id(SERVICE.scheduler().get_current_task_id()?)
    }

    #[allow(unused)]
    pub fn exit(&self) {
        assert!(!interrupt::is_enabled());
        assert_eq!(self.id, Task::current().unwrap().id);
        // Mark as dead
        {
            let mut live = self.live.lock();
            *live = false;
            self.live.notify_all()
        }
        // Remove from scheduler
        SERVICE.scheduler().remove_task(Task::current().unwrap().id);
        // Remove from process
        self.proc
            .upgrade()
            .unwrap()
            .tasks()
            .lock()
            .drain_filter(|t| *t == self.id);
    }
}

unsafe impl Send for Task {}
unsafe impl Sync for Task {}

impl PartialEq for Task {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Task {}
