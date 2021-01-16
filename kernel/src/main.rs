

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
#![feature(impl_trait_in_bindings)]
#![feature(type_alias_impl_trait)]
#![feature(const_impl_trait)]
#![feature(const_fn_fn_ptr_basics)]

extern crate alloc;
extern crate device_tree;

#[macro_use] mod log;
mod arch;
mod heap;
mod boot_driver;
mod task;
mod scheduler;
mod kernel_tasks;
mod memory;

use core::{mem, panic::PanicInfo};
use alloc::vec;
use kernel_tasks::{TestKernelTaskA, TestKernelTaskB};
use memory::physical::*;
use proton_kernel::BootInfo;
use scheduler::*;
use arch::*;
use task::*;


#[global_allocator]
static ALLOCATOR: heap::GlobalAllocator = heap::GlobalAllocator::new();

extern {
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
pub extern fn _start(boot_info: &mut BootInfo) -> isize {
    unsafe { zero_bss() }

    PHYSICAL_PAGE_RESOURCE.lock().init(boot_info.available_physical_memory);
    ALLOCATOR.init();

    let t = device_tree::DeviceTree::load(boot_info.device_tree).unwrap();
    TargetArch::init(&t);

    // return 233;
    let x = vec![ 233usize ];
    log!("Hello Proton! {:?}", x.as_ptr());

    let v = vec![1, 3, 5, 7, 9];
    log!("Test Alloc {:?} {:?}", v, v.as_ptr());

    let task = Task::create_kernel_task(box TestKernelTaskA);
    log!("[kernel: created kernel process: {:?}]", task.id());
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



// unsafe extern fn setup_vbar(ptr: u64) {
//     log!("efi_main: {:?}", efi_main as *const fn());
//     log!("handle_exception: {:?}", exception::handle_exception as *const fn());
//     log!("exception_handlers: {:?}", exception::exception_handlers as *const fn());
//     log!("exception_handlers real: {:#x}", ptr);
//     VBAR_EL1.set(ptr);
//     barrier::isb(barrier::SY);
// }

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