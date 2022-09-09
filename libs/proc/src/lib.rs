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
    pub const KERNEL: Self = Self(0);
}

#[derive(Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Clone, Copy)]
pub struct TaskId(pub usize);

impl TaskId {
    pub const NULL: Self = Self(0);
    pub const KERNEL: Self = Self(0);
}

/// Process manager
pub trait ProcessManager {
    fn new_state(&self) -> Box<dyn Any>;
    fn spawn(&self, t: Box<dyn Runnable>, mm: Box<dyn Any>) -> Arc<dyn Proc>;
    fn get_proc_by_id(&self, id: ProcId) -> Option<Arc<dyn Proc>>;
    fn current_proc(&self) -> Option<Arc<dyn Proc>>;
    fn get_task_by_id(&self, id: TaskId) -> Option<Arc<dyn Task>>;
    fn current_task(&self) -> Option<Arc<dyn Task>>;
}

#[derive(Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Clone, Copy)]
pub enum TaskState {
    Ready,
    Running,
    Blocked,
    Dead,
}

/// Abstruct task type
pub trait Task: Send + Sync {
    fn id(&self) -> TaskId;
    fn state(&self) -> &Monitor<bool>;
    fn context(&self) -> &dyn Any;
    fn proc(&self) -> Arc<dyn Proc>;
    fn sched(&self) -> &dyn Any;
    fn runnable(&self) -> &dyn Runnable;
}

/// Abstruct process type
pub trait Proc: Send + Sync + Any {
    fn id(&self) -> ProcId;
    fn fs(&self) -> &dyn Any;
    fn mm(&self) -> &dyn Any;
    fn tasks(&self) -> &Mutex<Vec<TaskId>>;
    fn spawn_task(self: Arc<Self>, task: Box<dyn Runnable>) -> Arc<dyn Task>;
    fn exit(&self);
    fn wait_for_completion(&self);
    // fn sbrk(&self, f: *const extern "C" fn());
}

/// Holds the execution code for a kernel task.
///
/// Unless jumping to the user mode, the program will remain in the kernel-space.
pub trait Runnable {
    fn run(&mut self) -> !;
}
