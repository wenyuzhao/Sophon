use core::{any::Any, sync::atomic::AtomicUsize};

use alloc::boxed::Box;
use atomic::Atomic;

use crate::proc::PID;

#[derive(Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Clone, Copy)]
pub struct TaskId(pub usize);

impl TaskId {
    pub const NULL: Self = Self(0);
}

pub trait Runnable {
    fn run(&mut self) -> !;
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunState {
    Ready = 0,
    Running,
    Blocked,
    Terminated,
}

unsafe impl bytemuck::Zeroable for RunState {
    fn zeroed() -> Self {
        RunState::Ready
    }
}

unsafe impl bytemuck::Pod for RunState {}

unsafe impl bytemuck::ZeroableInOption for RunState {}

unsafe impl bytemuck::PodInOption for RunState {}

#[repr(C)]
pub struct Task {
    pub id: TaskId,
    pub pid: PID,
    pub state: Atomic<RunState>,
    pub ticks: AtomicUsize,
    pub context: Box<dyn Any>,
    pub runnable: Option<Box<dyn Runnable>>,
}

unsafe impl bytemuck::Zeroable for TaskId {
    fn zeroed() -> Self {
        TaskId::NULL
    }
}

unsafe impl bytemuck::Pod for TaskId {}

unsafe impl bytemuck::ZeroableInOption for TaskId {}

unsafe impl bytemuck::PodInOption for TaskId {}
