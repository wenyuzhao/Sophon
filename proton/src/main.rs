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
#![feature(impl_trait_in_bindings)]
#![feature(type_alias_impl_trait)]
#![feature(const_impl_trait)]
#![feature(const_fn_fn_ptr_basics)]
#![feature(const_raw_ptr_to_usize_cast)]
#![feature(min_type_alias_impl_trait)]

extern crate alloc;
extern crate device_tree;

#[macro_use]
extern crate proton;

// mod log;
// mod arch;
// mod boot_driver;
// mod heap;
// mod kernel_tasks;
// // mod memory;
// mod scheduler;
// mod task;

use core::panic::PanicInfo;

use alloc::vec;
use proton::arch::{Arch, TargetArch};
use proton::kernel_tasks::{TestKernelTaskA, TestKernelTaskB};
use proton::memory::physical::{PhysicalPageResource, PHYSICAL_PAGE_RESOURCE};
use proton::scheduler::{AbstractScheduler, SCHEDULER};
use proton::task::Task;
use proton::BootInfo;

#[global_allocator]
static ALLOCATOR: proton::heap::GlobalAllocator = proton::heap::GlobalAllocator::new();

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
pub extern "C" fn _start(boot_info: &mut BootInfo) -> isize {
    unsafe { zero_bss() }

    // loop {}

    // Initialize physical memory and kernel heap
    PHYSICAL_PAGE_RESOURCE
        .lock()
        .init(boot_info.available_physical_memory);
    ALLOCATOR.init();

    // Initialize arch and boot drivers
    let t = device_tree::DeviceTree::load(boot_info.device_tree).unwrap();
    TargetArch::init(&t);
    // loop {}
    let x = vec![233usize];
    log!("Hello Proton! {:?}", x.as_ptr());

    ALLOCATOR.dump();

    let v = vec![1, 3, 5, 7, 9];
    log!("Test Alloc {:?} {:?}", v, v.as_ptr());

    let task = Task::create_kernel_task(box TestKernelTaskA);
    log!("[kernel: created kernel process: {:?}]", task.id());
    // loop {}
    let task = Task::create_kernel_task(box TestKernelTaskB);
    log!("[kernel: created kernel process: {:?}]", task.id());

    SCHEDULER.schedule();

    // let intc = t.find("/intc@8000000").unwrap();
    // let reg = intc.prop_raw("reg").unwrap();
    // log!("{:?}", intc);

    // drivers::gic::GIC.init_with_device_tree(&t);

    // unsafe {
    //     log!("[boot: enable all co-processors]");
    //     llvm_asm!("msr cpacr_el1, $0"::"r"(0xfffffff));
    //     setup_vbar(exception::exception_handlers as *const fn() as u64);
    //     crate::mm::paging::setup_ttbr();
    // }

    // unsafe {
    //     *(0xdeadbeed as *mut u8) = 0;
    // }
    // log!("CurrentEL: {}", CurrentEL.get() >> 2);

    loop {}
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
