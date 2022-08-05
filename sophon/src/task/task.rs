use super::runnable::Runnable;
use super::TaskId;
use crate::arch::Arch;
use crate::arch::ArchContext;
use crate::arch::TargetArch;
use crate::scheduler::AbstractScheduler;
use crate::scheduler::Scheduler;
use crate::scheduler::SCHEDULER;
use crate::*;
use alloc::boxed::Box;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicUsize, Ordering};

static TASK_ID_COUNT: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Clone, Copy)]
pub enum TaskState {
    Ready,
    Running,
    Blocked,
}

pub struct Task {
    pub id: TaskId,
    scheduler_state: <Scheduler as AbstractScheduler>::State,
    pub context: <TargetArch as Arch>::Context,
    pub proc: Arc<Proc>,
}

impl Task {
    #[inline]
    pub fn scheduler_state<S: AbstractScheduler>(&self) -> &S::State {
        let state: &<Scheduler as AbstractScheduler>::State = &self.scheduler_state;
        unsafe { core::mem::transmute(state) }
    }

    pub(super) fn create(proc: Arc<Proc>, t: Box<dyn Runnable>) -> Arc<Self> {
        let t = Box::into_raw(box t);
        let id = TaskId(TASK_ID_COUNT.fetch_add(1, Ordering::SeqCst));
        Arc::new(Task {
            id,
            context: <TargetArch as Arch>::Context::new(entry as _, t as *mut ()),
            scheduler_state: Default::default(),
            proc,
        })
    }

    pub fn by_id(id: TaskId) -> Option<Arc<Self>> {
        SCHEDULER.get_task_by_id(id)
    }

    pub fn current() -> Arc<Self> {
        SCHEDULER.get_current_task().unwrap()
    }

    pub fn current_opt() -> Option<Arc<Self>> {
        SCHEDULER.get_current_task()
    }

    pub fn get_context<C: ArchContext>(&self) -> &C {
        let ptr = &self.context as *const _;
        unsafe { &*(ptr as *const C) }
    }

    pub fn exit(&self) {
        SCHEDULER.remove_task(Task::current().id);
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

extern "C" fn entry(t: *mut Box<dyn Runnable>) -> ! {
    let mut t: Box<Box<dyn Runnable>> = unsafe { Box::from_raw(t) };
    t.run()
}
