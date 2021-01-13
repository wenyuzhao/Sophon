

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

use core::{mem, panic::PanicInfo};
use alloc::vec;
use kernel_tasks::{TestKernelTaskA, TestKernelTaskB};
use scheduler::*;
use arch::*;
use task::*;


#[global_allocator]
static ALLOCATOR: heap::GlobalAllocator = heap::GlobalAllocator::new();

static DEVICE_TREE: &'static [u8] = include_bytes!("../qemu-virt.dtb");

extern {
    static mut __bss_start: usize;
    static mut __bss_end: usize;
}

#[inline(never)]
unsafe fn zero_bss() {
    let start = (&mut __bss_start as *mut usize as usize & 0x0000ffff_ffffffff) as *mut u8;
    let end = (&mut __bss_end as *mut usize as usize & 0x0000ffff_ffffffff) as *mut u8;
    let mut cursor = start;
    while cursor < end {
        ::core::intrinsics::volatile_store(cursor, 0);
        cursor = cursor.offset(1);
    }
}

#[no_mangle]
pub extern fn _start(_argc: isize, _argv: *const *const u8) -> isize {
    unsafe { zero_bss() }

    ALLOCATOR.init();



    let t = device_tree::DeviceTree::load(DEVICE_TREE).unwrap();
    TargetArch::init(&t);

    // return 233;

    log!("Hello Proton!");

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