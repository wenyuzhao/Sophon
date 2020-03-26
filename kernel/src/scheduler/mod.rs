pub mod round_robin;

use crate::task::*;
use crate::AbstractKernel;
use alloc::boxed::Box;
use crate::arch::*;
use core::ops::{Deref, DerefMut};

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

pub trait SchedulerState: Clone + Default + ::core::fmt::Debug + Deref<Target=RunState> + DerefMut {}

pub trait AbstractScheduler: Sized + 'static {
    type State: SchedulerState;
    type Kernel: AbstractKernel;

    fn new() -> Self;

    fn register_new_task(&self, task: Box<Task<Self::Kernel>>) -> &'static mut Task<Self::Kernel>;
    fn remove_task(&self, id: TaskId);
    fn get_task_by_id(&self, id: TaskId) -> Option<&'static mut Task<Self::Kernel>>;
    fn get_current_task_id(&self) -> Option<TaskId>;
    fn get_current_task(&self) -> Option<&'static mut Task<Self::Kernel>>;

    fn mark_task_as_ready(&self, t: &'static mut Task<Self::Kernel>);

    fn unblock_sending_task(&self, id: TaskId, status: isize) {
        Self::uninterruptable(|| {
            let task = self.get_task_by_id(id).unwrap();
            assert!(**task.scheduler_state().borrow() == RunState::Sending);
            // Set response
            task.context.set_response_status(status);
            // Add this task to ready queue
            self.mark_task_as_ready(task)
        })
    }
    
    fn unblock_receiving_task(&self, id: TaskId, status: isize, m: Message) {
        Self::uninterruptable(|| {
            let task = self.get_task_by_id(id).unwrap();
            assert!(**task.scheduler_state().borrow() == RunState::Receiving);
            // Set response
            task.context.set_response_message(m);
            task.context.set_response_status(status);
            // Add this task to ready queue
            self.mark_task_as_ready(task)
        })
    }
    
    fn block_current_task_as_sending(&self) -> ! {
        Self::uninterruptable(|| {
            let task = self.get_current_task().unwrap();
            assert!(**task.scheduler_state().borrow() == RunState::Running);
            **task.scheduler_state().borrow_mut() = RunState::Sending;
            self.schedule();
        })
    }
    
    fn block_current_task_as_receiving(&self) -> ! {
        Self::uninterruptable(|| {
            let task = self.get_current_task().unwrap();
            assert!(**task.scheduler_state().borrow() == RunState::Running, "{:?} {:?}", task.id(), **task.scheduler_state().borrow());
            **task.scheduler_state().borrow_mut() = RunState::Receiving;
            self.schedule();
        })
    }

    // fn enqueue_current_task_as_ready(&self);
    fn schedule(&self) -> !;
    fn timer_tick(&self);

    #[inline]
    fn uninterruptable<R, F: FnOnce() -> R>(f: F) -> R {
        <<Self::Kernel as AbstractKernel>::Arch as AbstractArch>::Interrupt::uninterruptable(f)
    }
}

