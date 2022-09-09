use core::{any::Any, sync::atomic::AtomicUsize};

use alloc::{
    boxed::Box,
    collections::BTreeMap,
    sync::{Arc, Weak},
};
use atomic::Ordering;
use kernel_module::SERVICE;
use proc::{Proc, Runnable, TaskId};
use spin::{Lazy, Mutex};
use sync::Monitor;

pub static TASKS: Mutex<BTreeMap<TaskId, Arc<Task>>> = Mutex::new(BTreeMap::new());

pub struct Task {
    pub id: TaskId,
    pub context: Box<dyn Any>,
    pub proc: Weak<dyn Proc>,
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
        let task = Arc::new(Task {
            id,
            context,
            proc: Arc::downgrade(&proc),
            live: Lazy::new(|| Monitor::new(true)),
            sched: SERVICE.scheduler().new_state(),
            runnable,
        });
        TASKS.lock().insert(task.id, task.clone());
        task
    }

    #[inline(always)]
    pub fn by_id(id: TaskId) -> Option<Arc<Self>> {
        TASKS.lock().get(&id).cloned()
    }

    #[inline(always)]
    pub const fn as_dyn(self: Arc<Self>) -> Arc<dyn proc::Task> {
        self
    }

    #[inline(always)]
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
