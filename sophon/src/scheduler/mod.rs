pub mod monitor;
mod round_robin;

use crate::arch::Arch;
use alloc::{sync::Arc, vec::Vec};
use atomic::{Atomic, Ordering};
use core::fmt::Debug;
use core::ops::Deref;

use crate::{
    arch::TargetArch,
    task::{Task, TaskId},
};

/**
 *                        ___________
 *                       |           |
 *                       v           |
 * [CreateProcess] --> Ready ---> Running
 *                       ^           |
 *                       |           v
 *                       |___ Sending/Receiving
 *
 */
#[derive(Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Clone, Copy)]
#[repr(u8)]
pub enum RunState {
    Ready,
    Running,
    Sleeping,
}

pub trait AbstractSchedulerState: Default + Debug + Deref<Target = Atomic<RunState>> {}

pub trait AbstractScheduler: Sized + 'static {
    type State: AbstractSchedulerState;

    fn register_new_task(&self, task: Arc<Task>, affinity: Option<usize>) -> Arc<Task>;
    fn remove_task(&self, id: TaskId);
    fn get_task_by_id(&self, id: TaskId) -> Option<Arc<Task>>;
    fn get_current_task_id(&self) -> Option<TaskId>;
    fn get_current_task(&self) -> Option<Arc<Task>>;

    fn freeze_current_task(&self) {
        let _guard = interrupt::uninterruptible();
        let task = self.get_current_task().unwrap();
        assert_eq!(
            task.scheduler_state::<Self>().load(Ordering::SeqCst),
            RunState::Running
        );
        task.scheduler_state::<Self>()
            .store(RunState::Sleeping, Ordering::SeqCst);
        self.schedule();
    }

    fn wake_up(&self, t: Arc<Task>);

    fn schedule(&self) -> !;
    fn timer_tick(&self);
}

pub type Scheduler = impl AbstractScheduler;

static SCHEDULER_IMPL: round_robin::RoundRobinScheduler = round_robin::create();

pub static SCHEDULER: &'static Scheduler = &SCHEDULER_IMPL;

struct ProcessorLocalStorage<T: Default> {
    data: Vec<T>,
}

impl<T: Default> ProcessorLocalStorage<T> {
    pub fn new() -> Self {
        let len = TargetArch::num_cpus();
        Self {
            data: (0..len).map(|_| T::default()).collect(),
        }
    }

    pub fn get(&self, index: usize) -> &T {
        &self.data[index]
    }
}

impl<T: Default> Deref for ProcessorLocalStorage<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.data[TargetArch::current_cpu()]
    }
}
