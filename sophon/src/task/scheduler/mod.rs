mod round_robin;

use crate::arch::*;
use alloc::boxed::Box;
use core::ops::{Deref, DerefMut};

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
pub enum RunState {
    Ready,
    Running,
    Sending,
    Receiving,
}

pub trait AbstractSchedulerState:
    Clone + Default + ::core::fmt::Debug + Deref<Target = RunState> + DerefMut
{
}

pub trait AbstractScheduler: Sized + 'static {
    type State: AbstractSchedulerState;

    fn register_new_task(&self, task: Box<Task>) -> &'static mut Task;
    fn remove_task(&self, id: TaskId);
    fn get_task_by_id(&self, id: TaskId) -> Option<&'static mut Task>;
    fn get_current_task_id(&self) -> Option<TaskId>;
    fn get_current_task(&self) -> Option<&'static mut Task>;

    fn mark_task_as_ready(&self, t: &'static mut Task);

    fn unblock_sending_task(&self, id: TaskId, status: isize) {
        let _guard = interrupt::uninterruptable();
        let task = self.get_task_by_id(id).unwrap();
        assert!(**task.scheduler_state::<Self>().borrow() == RunState::Sending);
        // Set response
        task.context.set_response_status(status);
        // Add this task to ready queue
        self.mark_task_as_ready(task)
    }

    fn unblock_receiving_task(&self, id: TaskId, status: isize, m: Message) {
        let task = self.get_task_by_id(id).unwrap();
        assert!(**task.scheduler_state::<Self>().borrow() == RunState::Receiving);
        // Set response
        task.context.set_response_message(m);
        task.context.set_response_status(status);
        // Add this task to ready queue
        self.mark_task_as_ready(task)
    }

    fn block_current_task_as_sending(&self) -> ! {
        let _guard = interrupt::uninterruptable();
        let task = self.get_current_task().unwrap();
        assert!(**task.scheduler_state::<Self>().borrow() == RunState::Running);
        **task.scheduler_state::<Self>().borrow_mut() = RunState::Sending;
        self.schedule();
    }

    fn block_current_task_as_receiving(&self) -> ! {
        let _guard = interrupt::uninterruptable();
        let task = self.get_current_task().unwrap();
        assert!(
            **task.scheduler_state::<Self>().borrow() == RunState::Running,
            "{:?} {:?}",
            task.id(),
            **task.scheduler_state::<Self>().borrow()
        );
        **task.scheduler_state::<Self>().borrow_mut() = RunState::Receiving;
        self.schedule();
    }

    fn schedule(&self) -> !;
    fn timer_tick(&self);
}

pub type Scheduler = impl AbstractScheduler;

pub static SCHEDULER: Scheduler = round_robin::create();
