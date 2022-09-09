#![no_std]
#![feature(format_args_nl)]

extern crate alloc;

use core::any::Any;

use alloc::{boxed::Box, sync::Arc, vec::Vec};
use spin::Mutex;
use sync::Monitor;

#[derive(Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Clone, Copy)]
pub struct ProcId(pub usize);

impl ProcId {
    pub const NULL: Self = Self(0);
}

#[derive(Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Clone, Copy)]
pub struct TaskId(pub usize);

impl TaskId {
    pub const NULL: Self = Self(0);
}

/// Process manager
pub trait ProcessManager {
    /// Create a new process
    fn spawn(&self, t: Box<dyn Runnable>) -> Arc<dyn Proc>;
    /// Get a process by its id
    fn get_proc_by_id(&self, id: ProcId) -> Option<Arc<dyn Proc>>;
    /// Get the current process
    fn current_proc(&self) -> Option<Arc<dyn Proc>>;
    /// Get the current process id
    fn current_proc_id(&self) -> Option<ProcId>;
    /// Get a task by its id
    fn get_task_by_id(&self, id: TaskId) -> Option<Arc<dyn Task>>;
    /// Get the current task
    fn current_task(&self) -> Option<Arc<dyn Task>>;
}

/// Abstruct task type
pub trait Task: Send + Sync {
    /// Get task id
    fn id(&self) -> TaskId;
    /// Get task running state
    fn state(&self) -> &Monitor<bool>;
    /// Arch-dependent context for context switching
    fn context(&self) -> &dyn Any;
    /// The process that owns this task
    fn proc(&self) -> Arc<dyn Proc>;
    /// Task scheduler state
    fn sched(&self) -> &dyn Any;
    /// The task runnable
    fn runnable(&self) -> &dyn Runnable;
}

/// Abstruct process type
pub trait Proc: Send + Sync + Any {
    /// Get process id
    fn id(&self) -> ProcId;
    /// Get process vfs state
    fn fs(&self) -> &dyn Any;
    /// Get process memory state
    fn mm(&self) -> &dyn Any;
    /// Get all the tasks in this process
    fn tasks(&self) -> &Mutex<Vec<TaskId>>;
    /// Spawn a task
    fn spawn_task(self: Arc<Self>, task: Box<dyn Runnable>) -> Arc<dyn Task>;
    /// Exit the process
    fn exit(&self);
    /// Wait for the process to complete
    fn wait_for_completion(&self);
}

/// Holds the execution code for a kernel task.
///
/// Unless jumping to the user mode, the program will remain in the kernel-space.
pub trait Runnable {
    fn run(&mut self) -> !;
}
