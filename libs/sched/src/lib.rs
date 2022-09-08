#![no_std]

extern crate alloc;

use alloc::boxed::Box;
use core::any::Any;
use proc::TaskId;

/// Task scheduler.
pub trait Scheduler: Send + Sync + 'static {
    /// Create a new per-task state.
    fn new_state(&self) -> Box<dyn Any>;
    /// Get current task id.
    fn get_current_task_id(&self) -> Option<TaskId>;
    /// Register a task.
    fn register_new_task(&self, task: TaskId);
    /// Dereference a task.
    fn remove_task(&self, task: TaskId);
    /// Sleep the current task.
    fn sleep(&self);
    /// Wake up a task.
    fn wake_up(&self, task: TaskId);
    /// Switch to another task.
    fn schedule(&self) -> !;
    /// Tick the timer.
    fn timer_tick(&self) -> !;
}

#[derive(Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Clone, Copy)]
#[repr(u8)]
pub enum RunState {
    Ready,
    Running,
    Sleeping,
}
