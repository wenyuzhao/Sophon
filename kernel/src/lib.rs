#![feature(const_fn)]
#![feature(format_args_nl)]
#![feature(associated_type_defaults)]
#![feature(box_syntax)]
#![no_std]

extern crate alloc;
extern crate elf_rs;

pub mod arch;
pub mod memory;
mod utils;
#[macro_use]
pub mod debug;
pub mod task;
pub mod scheduler;
pub mod ipc;
pub mod kernel_process;

use arch::*;
use scheduler::AbstractScheduler;
use proton::lazy::Lazy;
use ipc::IPCController;
use kernel_process::system::System;
use kernel_process::user::UserTask;
use task::Task;



static INIT_ELF: &'static [u8] = include_bytes!("../../target/aarch64-proton/debug/init");

pub struct KernelGlobal<K: AbstractKernel> {
    pub scheduler: Lazy<K::Scheduler>,
    pub ipc: IPCController<K>,
}

pub trait AbstractKernel: Sized + 'static {
    type Arch: AbstractArch;
    type Scheduler: AbstractScheduler<Kernel=Self>;
    type Global = KernelGlobal<Self>;

    const INITIAL_GLOBAL: KernelGlobal<Self> = KernelGlobal {
        scheduler: Lazy::new(Self::Scheduler::new),
        ipc: IPCController::new(),
    };

    fn global() -> &'static KernelGlobal<Self>;

    fn start() -> ! {
        debug!(Self: "Hello, Raspberry PI!");
        // Initialize kernel heap
        <Self::Arch as AbstractArch>::Heap::init();
        debug!(Self: "[kernel: kernel heap initialized]");
        debug!(Self: " - test allocation -> {}", box 233);
        <Self::Arch as AbstractArch>::Interrupt::init();
        debug!(Self: "[kernel: interrupt initialized]");
        ipc::init::<Self>();
        debug!(Self: "[kernel: ipc initialized]");
        <Self::Arch as AbstractArch>::Timer::init();
        debug!(Self: "[kernel: timer initialized]");

        
        // let task = KernelProcess::<Self>::spawn();
        let task = Task::<Self>::create_kernel_task(box System::<Self>::new());
        debug!(Self: "Created kernel process: {:?}", task.id());
        let task = Task::<Self>::create_kernel_task(box UserTask::<Self>::new(INIT_ELF));
        debug!(Self: "Created init process: {:?}", task.id());
        let task = Task::<Self>::create_kernel_task(Self::Arch::create_idle_task());
        debug!(Self: "Created idle process: {:?}", task.id());

        Self::global().scheduler.schedule();
    }
}
