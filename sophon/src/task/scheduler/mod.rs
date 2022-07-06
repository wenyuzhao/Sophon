mod round_robin;

use crate::arch::*;
use alloc::sync::Arc;
use atomic::{Atomic, Ordering};
use core::fmt::Debug;
use core::ops::Deref;

use super::{task::Task, Message, TaskId};

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

    fn unblock_sending_task(&self, id: TaskId, status: isize) {
        let _guard = interrupt::uninterruptible();
        let task = self.get_task_by_id(id).unwrap();
        assert_eq!(
            task.scheduler_state::<Self>().load(Ordering::SeqCst),
            RunState::Sending
        );
        // Set response
        task.context.set_response_status(status);
        // Add this task to ready queue
        self.mark_task_as_ready(task)
    }

    fn unblock_receiving_task(&self, id: TaskId, status: isize, m: Message) {
        let task = self.get_task_by_id(id).unwrap();
        assert_eq!(
            task.scheduler_state::<Self>().load(Ordering::SeqCst),
            RunState::Receiving
        );
        // Set response
        task.context.set_response_message(m);
        task.context.set_response_status(status);
        // Add this task to ready queue
        self.mark_task_as_ready(task)
    }

    fn block_current_task_as_sending(&self) -> ! {
        let _guard = interrupt::uninterruptible();
        let task = self.get_current_task().unwrap();
        assert_eq!(
            task.scheduler_state::<Self>().load(Ordering::SeqCst),
            RunState::Running
        );
        task.scheduler_state::<Self>()
            .store(RunState::Sending, Ordering::SeqCst);
        self.schedule();
    }

    fn block_current_task_as_receiving(&self) -> ! {
        let _guard = interrupt::uninterruptible();
        let task = self.get_current_task().unwrap();
        assert_eq!(
            task.scheduler_state::<Self>().load(Ordering::SeqCst),
            RunState::Running
        );
        task.scheduler_state::<Self>()
            .store(RunState::Receiving, Ordering::SeqCst);
        self.schedule();
    }
}

pub type Scheduler = impl AbstractScheduler;

static SCHEDULER_IMPL: round_robin::RoundRobinScheduler = round_robin::create();

pub static SCHEDULER: &'static Scheduler = &SCHEDULER_IMPL;
