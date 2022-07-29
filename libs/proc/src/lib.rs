#![no_std]
#![feature(format_args_nl)]

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
