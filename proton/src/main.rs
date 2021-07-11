#![no_std]
#![no_main]
#![feature(global_asm)]
#![feature(alloc_error_handler)]
#![feature(format_args_nl)]
#![feature(core_intrinsics)]
#![feature(box_syntax)]
#![feature(new_uninit)]
#![feature(never_type)]
#![feature(step_trait_ext)]
#![feature(const_impl_trait)]
#![feature(const_fn_fn_ptr_basics)]
#![feature(const_raw_ptr_to_usize_cast)]
#![feature(min_type_alias_impl_trait)]

extern crate alloc;
extern crate device_tree;

#[macro_use]
extern crate proton;

use core::panic::PanicInfo;

use alloc::vec;
use proton::arch::{Arch, TargetArch};
use proton::kernel_tasks::system::{Idle, System};
use proton::kernel_tasks::user::UserTask;
use proton::memory::kernel::heap::{KernelHeapAllocator, KERNEL_HEAP};
use proton::memory::kernel::mapper::KERNEL_MEMORY_MAPPER;
use proton::memory::physical::{PhysicalPageResource, PHYSICAL_PAGE_RESOURCE};
use proton::scheduler::ipc::IPC;
use proton::scheduler::task::Task;
use proton::scheduler::{AbstractScheduler, SCHEDULER};
use proton::BootInfo;

#[global_allocator]
static ALLOCATOR: KernelHeapAllocator = KernelHeapAllocator;

extern "C" {
    static mut __bss_start: u8;
    static mut __bss_end: u8;
}

#[cfg(debug_assertions)]
const INIT: &'static [u8] = include_bytes!("../../target/aarch64-proton/debug/init");
#[cfg(not(debug_assertions))]
const INIT: &'static [u8] = include_bytes!("../../target/aarch64-proton/release/init");

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
pub extern "C" fn _start(boot_info: &mut BootInfo) -> isize {
    unsafe { zero_bss() }

    // Initialize physical memory and kernel heap
    PHYSICAL_PAGE_RESOURCE
        .lock()
        .init(boot_info.available_physical_memory);
    KERNEL_MEMORY_MAPPER.init();
    KERNEL_HEAP.init();

    // Initialize arch and boot drivers
    let t = device_tree::DeviceTree::load(boot_info.device_tree).unwrap();
    TargetArch::init(&t);
    // loop {}
    let x = vec![233usize];
    log!("Hello Proton! {:?}", x.as_ptr());

    KERNEL_HEAP.dump();

    let v = vec![1, 3, 5, 7, 9];
    log!("Test Alloc {:?} {:?}", v, v.as_ptr());

    IPC::init();
    log!("[kernel: ipc initialized]");

    let task = Task::create_kernel_task(box System::new());
    log!("[kernel: created kernel process: {:?}]", task.id());

    let task = Task::create_kernel_task(box Idle);
    log!("[kernel: created kernel process: {:?}]", task.id());

    let task = Task::create_kernel_task(box UserTask::new(INIT));
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
