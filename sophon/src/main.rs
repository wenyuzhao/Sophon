#![allow(incomplete_features)]
#![feature(format_args_nl)]
#![feature(box_syntax)]
#![feature(core_intrinsics)]
#![feature(impl_trait_in_bindings)]
#![feature(min_type_alias_impl_trait)]
#![feature(step_trait)]
#![feature(global_asm)]
#![feature(asm)]
#![feature(const_impl_trait)]
#![feature(const_fn_fn_ptr_basics)]
#![feature(const_trait_impl)]
#![feature(const_generics)]
#![feature(const_fn_trait_bound)]
#![feature(const_btree_new)]
#![feature(alloc_error_handler)]
#![no_std]
#![no_main]

extern crate alloc;
extern crate elf_rs;

#[macro_use]
pub mod utils;
#[macro_use]
pub mod log;
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
use crate::kernel_tasks::user::UserTask;
use crate::kernel_tasks::Idle;
use crate::memory::kernel::{KernelHeapAllocator, KERNEL_HEAP};
use crate::memory::physical::PHYSICAL_MEMORY;
use crate::task::scheduler::{AbstractScheduler, SCHEDULER};
use crate::task::Task;
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
    let mut cursor = start;
    while cursor < end {
        ::core::intrinsics::volatile_store(cursor, 0);
        cursor = cursor.offset(1);
    }
}

#[no_mangle]
pub extern "C" fn _start(boot_info: &BootInfo) -> isize {
    if let Some(uart) = boot_info.uart {
        unsafe { log::BOOT_LOG.set_mmio_address(uart) }
    }
    boot_log!("SOPHON");
    boot_log!("boot_info @ {:?} {:?}", boot_info as *const _, unsafe {
        *(boot_info as *const _ as *const usize)
    });
    boot_log!("device_tree @ {:?}", boot_info.device_tree.as_ptr_range());
    boot_log!(
        "available_physical_memory @ {:?}",
        boot_info.available_physical_memory.as_ptr_range()
    );
    unsafe { zero_bss() }
    boot_log!("zero_bss done");

    // Initialize physical memory and kernel heap
    PHYSICAL_MEMORY.init(boot_info.available_physical_memory);
    boot_log!("PHYSICAL_MEMORY done");
    KERNEL_HEAP.init();
    boot_log!("KERNEL_HEAP done");

    // Initialize arch and boot drivers
    let fdt = Fdt::new(boot_info.device_tree).unwrap();
    boot_log!("fdt loaded");
    TargetArch::init(&fdt);
    boot_log!("TargetArch done");

    log!("Hello Sophon!");

    let v = vec![1, 3, 5, 7, 9];
    log!("[kernel: test-alloc] {:?} @ {:?}", v, v.as_ptr());

    task::ipc::init();
    log!("[kernel: ipc initialized]");

    schemes::register_kernel_schemes();
    log!("[kernel: schemes initialized]");

    InitFS::deserialize(boot_info.init_fs);
    log!("[kernel: initfs initialized]");

    let task = Task::create_kernel_task(box Idle);
    log!("[kernel: created kernel process: {:?}]", task.id());

    // let program = InitFS::get().get_file("/scheme_test");
    // let task = Task::create_kernel_task(box UserTask::new(program));
    // log!("[kernel: created scheme_test process: {:?}]", task.id());

    let program = InitFS::get().get_file("/init");
    let task = Task::create_kernel_task(box UserTask::new(program));
    log!("[kernel: created init process: {:?}]", task.id());

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
