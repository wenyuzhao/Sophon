#![feature(const_fn)]
#![feature(format_args_nl)]
#![feature(associated_type_defaults)]
#![feature(box_syntax)]
#![feature(core_intrinsics)]
#![feature(never_type)]
#![feature(const_fn_transmute)]
#![feature(const_raw_ptr_deref)]
#![feature(const_panic)]
#![feature(specialization)]
#![feature(const_mut_refs)]
#![feature(impl_trait_in_bindings)]
#![feature(min_type_alias_impl_trait)]
#![no_std]

extern crate alloc;
extern crate elf_rs;

// #[macro_use]
// pub mod debug;
// pub mod arch;
// pub mod memory;
#[macro_use]
pub mod utils;
// pub mod task;
// pub mod scheduler;
// pub mod ipc;
// pub mod kernel_process;
pub mod page_table;

use core::ops::Range;

// use arch::*;
// use scheduler::AbstractScheduler;
use proton::{lazy::Lazy, memory::{Address, Frame, P, Page}};
// use ipc::IPCController;
// use kernel_process::system::System;
// use kernel_process::user::UserTask;
// use task::Task;



// pub struct KernelGlobal<K: AbstractKernel> {
//     pub scheduler: Lazy<K::Scheduler>,
//     pub ipc: IPCController<K>,
// }

// pub trait AbstractKernel: Sized + 'static {
//     type Arch: AbstractArch;
//     type Scheduler: AbstractScheduler<Kernel=Self>;
//     type Global = KernelGlobal<Self>;

//     const INITIAL_GLOBAL: KernelGlobal<Self> = KernelGlobal {
//         scheduler: Lazy::new(Self::Scheduler::new),
//         ipc: IPCController::new(),
//     };

//     fn global() -> &'static KernelGlobal<Self>;

//     fn start() -> ! {
//         debug!(Self: "Hello, Raspberry PI!");
//         // Initialize kernel heap
//         <Self::Arch as AbstractArch>::Heap::init();
//         debug!(Self: "[kernel: kernel heap initialized]");
//         debug!(Self: " - test allocation -> {}", box 233);
//         <Self::Arch as AbstractArch>::Interrupt::init();
//         debug!(Self: "[kernel: interrupt initialized]");
//         ipc::init::<Self>();
//         debug!(Self: "[kernel: ipc initialized]");
//         <Self::Arch as AbstractArch>::Timer::init();
//         debug!(Self: "[kernel: timer initialized]");
//         debug!(Self: "[kernel: timer initialized {}]", <Self::Arch as AbstractArch>::Interrupt::is_enabled());


//         // let task = Task::<Self>::create_kernel_task(box System::<Self>::new());
//         // debug!(Self: "[kernel: created kernel process: {:?}]", task.id());
//         // let task = Task::<Self>::create_kernel_task(Self::Arch::create_idle_task());
//         // debug!(Self: "[kernel: created idle process: {:?}]", task.id());

//         // // Load init.rd
//         // // let initrd_address = Arch::load_initrd();
//         // // Start ramfs driver
//         // // let task = Task::<Self>::create_kernel_task(box UserTask::<Self>::new(EMMC_ELF));
//         // // debug!(Self: "[kernel: created emmc process: {:?}]", task.id());

//         // // Load & start init process
//         // let task = Task::<Self>::create_kernel_task(box UserTask::<Self>::new(
//         //     <Self::Arch as AbstractArch>::BootImage::get("init").unwrap()
//         // ));
//         // debug!(Self: "[kernel: created init process: {:?}]", task.id());

//         // let _task = Task::<Self>::create_kernel_task2(box UserTask::<Self>::new(
//         //     <Self::Arch as AbstractArch>::BootImage::get("init").unwrap()
//         // ));

//         // debug!(Self: "[kernel: created emmc process: {:?}]", task.id());

//         Self::global().scheduler.schedule();
//     }
// }

pub struct BootInfo {
    pub available_physical_memory: &'static [Range<Frame>],
    pub device_tree: &'static [u8],
}