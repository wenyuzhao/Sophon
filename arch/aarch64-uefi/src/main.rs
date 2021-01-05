#![no_std]
#![no_main]
#![feature(global_asm)]
#![feature(llvm_asm)]
#![feature(alloc_error_handler)]
#![feature(format_args_nl)]
#![feature(core_intrinsics)]
#![feature(box_syntax)]
#![feature(new_uninit)]
#![feature(never_type)]
#![feature(step_trait_ext)]
#![feature(const_fn_transmute)]
#![feature(const_in_array_repeat_expressions)]

extern crate alloc;
extern crate bitflags;
use core::panic::PanicInfo;
use cortex_a::{asm, regs::*, barrier};
use uefi::prelude::*;
#[macro_use] mod log;
mod arch;
mod exception;
mod interrupt;
mod timer;
mod bootimage;
mod idle;
mod heap;
mod mm;
mod gic;
mod context;

static mut SYSTEM_TABLE: Option<SystemTable<Boot>> = None;

fn system_table() -> &'static SystemTable<Boot> {
    unsafe { SYSTEM_TABLE.as_ref().unwrap() }
}

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

#[no_mangle]
pub extern "C" fn efi_main(_image: Handle, st: SystemTable<Boot>) -> Status {
    unsafe { SYSTEM_TABLE = Some(st); }

    unsafe {
        // unsafe { llvm_asm!("msr daifset, #2") };
        unsafe { llvm_asm!("msr daifclr, #2") };
        log!("exception_handlers: {:?}", &exception::exception_handlers as *const _);
        VBAR_EL1.set(0);
        barrier::isb(barrier::SY);
    }
    log!("CurrentEL: {}", CurrentEL.get() >> 2);
    log!("Firmware vendor: {}", system_table().firmware_vendor());
    log!("Firmware revision: {:?}", system_table().firmware_revision());
    log!("UEFI revision: {:?}", system_table().uefi_revision());
    let mut buf = [0u8; 4096];
    let (k, i) = system_table().boot_services().memory_map(&mut buf).unwrap().unwrap();
    log!("mmap key: {:?}", k);
    for i in i {
        log!("> mem phys_start={:?} virt_start={:?} pages={:?}", i.phys_start as *const (), i.virt_start as *const (), i.page_count);
        log!("{:?}", i);
    }
    Kernel::start();
}

#[panic_handler]
fn panic(_panic: &PanicInfo<'_>) -> ! {
    log!("PANIC!");
    loop {}
}

#[no_mangle]
pub extern "C" fn __chkstk() {}

#[alloc_error_handler]
fn alloc_error_handler(layout: ::alloc::alloc::Layout) -> ! {
    panic!("Allocation error: {:?}", layout)
}