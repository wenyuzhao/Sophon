use crate::task::{Task, TaskId};
use alloc::{boxed::Box, collections::BTreeMap, sync::Arc};
use core::any::Any;
use spin::Mutex;

static mut SCHEDULER_IMPL: &'static dyn sched::Scheduler = &UnimplementedScheduler;

pub static SCHEDULER: Scheduler = Scheduler::new();

pub struct Scheduler {
    tasks: Mutex<BTreeMap<TaskId, Arc<Task>>>,
}

impl Scheduler {
    pub const fn new() -> Self {
        Self {
            tasks: Mutex::new(BTreeMap::new()),
        }
    }

    #[inline(always)]
    fn sched(&self) -> &'static dyn sched::Scheduler {
        unsafe { &*SCHEDULER_IMPL }
    }

    pub fn set_scheduler(&self, scheduler: &'static dyn sched::Scheduler) {
        unsafe {
            SCHEDULER_IMPL = scheduler;
        }
    }

    pub fn new_state(&self) -> Box<dyn Any> {
        self.sched().new_state()
    }

    pub fn get_current_task_id(&self) -> Option<TaskId> {
        self.sched().get_current_task_id()
    }

    pub fn register_new_task(&self, task: Arc<Task>, affinity: Option<usize>) -> Arc<Task> {
        self.tasks.lock().insert(task.id, task.clone());
        self.sched().register_new_task(task.id, affinity);
        task
    }

    pub fn remove_task(&self, task: TaskId) {
        self.tasks.lock().remove(&task);
        self.sched().remove_task(task)
    }

    pub fn sleep(&self) {
        self.sched().sleep()
    }

    pub fn wake_up(&self, task: TaskId) {
        self.sched().wake_up(task)
    }

    pub fn schedule(&self) -> ! {
        self.sched().schedule()
    }

    pub fn timer_tick(&self) {
        self.sched().timer_tick()
    }

    pub fn get_task_by_id(&self, id: TaskId) -> Option<Arc<Task>> {
        let _guard = interrupt::uninterruptible();
        let tasks = self.tasks.lock();
        let task = tasks.get(&id)?;
        Some(task.clone())
    }

    pub fn get_current_task(&self) -> Option<Arc<Task>> {
        let _guard = interrupt::uninterruptible();
        self.get_task_by_id(self.get_current_task_id()?)
    }
}

struct UnimplementedScheduler;

impl sched::Scheduler for UnimplementedScheduler {
    fn new_state(&self) -> Box<dyn Any> {
        unimplemented!()
    }
    fn get_current_task_id(&self) -> Option<TaskId> {
        unimplemented!()
    }
    fn register_new_task(&self, _task: TaskId, _affinity: Option<usize>) {
        unimplemented!()
    }
    fn remove_task(&self, _task: TaskId) {
        unimplemented!()
    }
    fn sleep(&self) {
        unimplemented!()
    }
    fn wake_up(&self, _task: TaskId) {
        unimplemented!()
    }
    fn schedule(&self) -> ! {
        unimplemented!()
    }
    fn timer_tick(&self) {
        unimplemented!()
    }
}
