#![feature(asm)]
#![feature(format_args_nl)]
#![feature(global_asm)]
#![feature(panic_info_message)]
#![feature(core_intrinsics)]
#![feature(stmt_expr_attributes)]
#![feature(naked_functions)]
#![feature(const_fn)]
#![feature(const_raw_ptr_to_usize_cast)]
#![feature(step_trait)]
#![feature(const_transmute)]
#![feature(box_syntax)]
#![feature(alloc_error_handler)]
#![feature(new_uninit)]
#![feature(type_alias_impl_trait)]
#![feature(internal_uninit_const)]
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

pub mod start;
pub mod gic;
pub mod interrupt;
pub mod exception;
pub mod timer;
pub mod context;
pub mod mm;
mod heap;
pub mod uart;
pub mod constants;
mod arch;

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