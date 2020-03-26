#![feature(asm)]
#![feature(format_args_nl)]
#![feature(global_asm)]
#![feature(core_intrinsics)]
#![feature(naked_functions)]
#![feature(step_trait)]
#![feature(const_transmute)]
#![feature(box_syntax)]
#![feature(alloc_error_handler)]
#![feature(new_uninit)]
#![feature(never_type)]
#![feature(const_in_array_repeat_expressions)]
#![allow(dead_code)]
#![no_std]
#![no_main]

extern crate spin;
extern crate cortex_a;
extern crate proton;
#[macro_use]
extern crate bitflags;
#[allow(unused)]
#[macro_use]
extern crate alloc;
extern crate device_tree;
#[macro_use]
extern crate proton_kernel;

mod start;
mod gic;
mod interrupt;
mod exception;
mod timer;
mod context;
mod mm;
mod heap;
mod uart;
mod constants;
mod arch;
mod idle;

use proton_kernel::AbstractKernel;
use proton_kernel::scheduler::round_robin::RoundRobinScheduler;
use arch::AArch64;



#[global_allocator]
static ALLOCATOR: heap::GlobalAllocator = heap::GlobalAllocator::new();

static KERNEL: Kernel = Kernel {
    global: <Kernel as AbstractKernel>::INITIAL_GLOBAL,
};



pub struct Kernel {
    global: <Self as AbstractKernel>::Global,
}

impl AbstractKernel for Kernel {
    type Arch = AArch64;
    type Scheduler = RoundRobinScheduler<Self>;

    fn global() -> &'static Self::Global {
        &KERNEL.global
    }
}



#[panic_handler]
fn panic(info: &::core::panic::PanicInfo) -> ! {
    debug!(Kernel: "{}", info);
    loop {}
}


#[alloc_error_handler]
fn alloc_error_handler(layout: ::alloc::alloc::Layout) -> ! {
    panic!("Allocation error: {:?}", layout)
}