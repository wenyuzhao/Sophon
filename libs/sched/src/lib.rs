#![no_std]

extern crate alloc;

use alloc::boxed::Box;
use core::any::Any;
use proc::TaskId;

pub trait Scheduler: Send + Sync + 'static {
    // Tasks
    fn new_state(&self) -> Box<dyn Any>;
    fn get_current_task_id(&self) -> Option<TaskId>;
    fn register_new_task(&self, task: TaskId);
    fn remove_task(&self, task: TaskId);
    // Scheduling
    fn sleep(&self);
    fn wake_up(&self, task: TaskId);
    fn schedule(&self) -> !;
    fn timer_tick(&self) -> !;
}

#[derive(Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Clone, Copy)]
#[repr(u8)]
pub enum RunState {
    Ready,
    Running,
    Sleeping,
}
