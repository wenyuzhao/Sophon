use crate::task::TaskId;
use alloc::{collections::BTreeMap, sync::Arc};
use core::ops::Deref;
use spin::Mutex;

static mut SCHEDULER_IMPL: Option<&'static dyn sched::Scheduler> = None;

pub static SCHEDULER: Scheduler = Scheduler::new();

pub struct Scheduler {
    // tasks: Mutex<BTreeMap<TaskId, Arc<Task>>>,
}

impl Scheduler {
    pub const fn new() -> Self {
        Self {
            // tasks: Mutex::new(BTreeMap::new()),
        }
    }

    pub fn set_scheduler(&self, scheduler: &'static dyn sched::Scheduler) {
        unsafe {
            SCHEDULER_IMPL = Some(scheduler);
        }
    }

    // pub fn register_new_task(&self, task: Arc<Task>) -> Arc<Task> {
    //     self.tasks.lock().insert(task.id, task.clone());
    //     self.deref().register_new_task(task.id);
    //     task
    // }

    // pub fn remove_task(&self, task: TaskId) {
    //     self.tasks.lock().remove(&task);
    //     self.deref().remove_task(task)
    // }

    // pub fn get_task_by_id(&self, id: TaskId) -> Option<Arc<Task>> {
    //     let _guard = interrupt::uninterruptible();
    //     let tasks = self.tasks.lock();
    //     let task = tasks.get(&id)?;
    //     Some(task.clone())
    // }

    // pub fn get_current_task(&self) -> Option<Arc<Task>> {
    //     let _guard = interrupt::uninterruptible();
    //     self.get_task_by_id(self.get_current_task_id()?)
    // }
}

impl Deref for Scheduler {
    type Target = dyn sched::Scheduler;
    fn deref(&self) -> &Self::Target {
        unsafe { SCHEDULER_IMPL.unwrap_unchecked() }
    }
}
