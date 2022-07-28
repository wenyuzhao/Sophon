#![allow(incomplete_features)]
#![feature(format_args_nl)]
#![feature(box_syntax)]
#![feature(core_intrinsics)]
#![feature(step_trait)]
#![feature(const_trait_impl)]
#![feature(const_btree_new)]
#![feature(alloc_error_handler)]
#![feature(const_mut_refs)]
#![feature(adt_const_params)]
#![feature(generic_const_exprs)]
#![feature(type_alias_impl_trait)]
#![no_std]
#![no_main]

extern crate alloc;
#[macro_use]
extern crate log;

#[macro_use]
pub mod utils;
pub mod arch;
pub mod boot_driver;
#[path = "../init-fs.rs"]
pub mod initfs;
pub mod kernel_tasks;
pub mod memory;
pub mod schemes;
pub mod task;

use core::panic::PanicInfo;

use crate::arch::{Arch, TargetArch};
use crate::initfs::InitFS;
use crate::kernel_tasks::system::System;
use crate::kernel_tasks::Idle;
use crate::memory::kernel::{KernelHeapAllocator, KERNEL_HEAP};
use crate::memory::physical::PHYSICAL_MEMORY;
use crate::task::scheduler::{AbstractScheduler, SCHEDULER};
use crate::task::Proc;
use alloc::vec;
use boot::BootInfo;
use fdt::Fdt;

#[global_allocator]
static ALLOCATOR: KernelHeapAllocator = KernelHeapAllocator;

extern "C" {
    static mut __bss_start: u8;
    static mut __bss_end: u8;
}

#[inline(never)]
unsafe fn zero_bss() {
    let start = &mut __bss_start as *mut u8;
    let end = &mut __bss_end as *mut u8;
    let bytes = end.offset_from(start);
    core::ptr::write_bytes(start, 0, bytes as _)
}

fn display_banner() {
    println!(r"");
    println!(r" ____ ____ ___  _  _ ____ _  _    ____ ____ ");
    println!(r" [__  |  | |__] |__| |  | |\ |    |  | [__  ");
    println!(r" ___] |__| |    |  | |__| | \|    |__| ___] ");
    println!(r"");
}

#[no_mangle]
pub extern "C" fn _start(boot_info: &BootInfo) -> isize {
    unsafe { zero_bss() }
    if let Some(uart) = boot_info.uart {
        utils::boot_log::init(uart);
    }
    display_banner();
    log!("boot_info @ {:?} {:?}", boot_info as *const _, unsafe {
        *(boot_info as *const _ as *const usize)
    });
    log!("device_tree @ {:?}", boot_info.device_tree.as_ptr_range());
    log!(
        "available_physical_memory @ {:?}",
        boot_info.available_physical_memory.as_ptr_range()
    );

    // Initialize physical memory and kernel heap
    PHYSICAL_MEMORY.init(boot_info.available_physical_memory);
    log!("PHYSICAL_MEMORY done");
    KERNEL_HEAP.init();
    log!("KERNEL_HEAP done");

    // Initialize arch and boot drivers
    let fdt = Fdt::new(boot_info.device_tree).unwrap();
    log!("fdt loaded");
    TargetArch::init(&fdt);
    log!("TargetArch done");

    log!("Hello Sophon!");

    let v = vec![1, 3, 5, 7, 9];
    log!("[kernel: test-alloc] {:?} @ {:?}", v, v.as_ptr());

    task::ipc::init();
    log!("[kernel: ipc initialized]");

    schemes::register_kernel_schemes();
    log!("[kernel: schemes initialized]");

    InitFS::deserialize(boot_info.init_fs);
    log!("[kernel: initfs initialized]");

    let proc = Proc::spawn(box Idle);
    log!("[kernel: created idle process: {:?}]", proc.id);

    let proc = Proc::spawn(box System);
    log!("[kernel: created system process: {:?}]", proc.id);

    TargetArch::interrupt().start_timer();
    log!("[kernel: timer started]");

    SCHEDULER.schedule();
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    log!("{}", info);
    loop {}
}

#[no_mangle]
pub extern "C" fn __chkstk() {}

#[alloc_error_handler]
fn alloc_error_handler(layout: ::alloc::alloc::Layout) -> ! {
    panic!("Allocation error: {:?}", layout)
}
