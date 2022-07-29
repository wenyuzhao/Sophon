mod round_robin;

use alloc::sync::Arc;
use atomic::{Atomic, Ordering};
use core::fmt::Debug;
use core::ops::Deref;

use super::{task::Task, TaskId};

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
    Sending,
    Receiving,
}

pub trait AbstractSchedulerState: Default + Debug + Deref<Target = Atomic<RunState>> {}

pub trait AbstractScheduler: Sized + 'static {
    type State: AbstractSchedulerState;

    fn register_new_task(&self, task: Arc<Task>) -> Arc<Task>;
    fn remove_task(&self, id: TaskId);
    fn get_task_by_id(&self, id: TaskId) -> Option<Arc<Task>>;
    fn get_current_task_id(&self) -> Option<TaskId>;
    fn get_current_task(&self) -> Option<Arc<Task>>;

    fn mark_task_as_ready(&self, t: Arc<Task>);

    fn schedule(&self) -> !;
    fn timer_tick(&self);
}

pub type Scheduler = impl AbstractScheduler;

static SCHEDULER_IMPL: round_robin::RoundRobinScheduler = round_robin::create();

pub static SCHEDULER: &'static Scheduler = &SCHEDULER_IMPL;
