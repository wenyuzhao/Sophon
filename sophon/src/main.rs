#![no_std]
#![no_main]
#![feature(global_asm)]
#![feature(alloc_error_handler)]
#![feature(format_args_nl)]
#![feature(core_intrinsics)]
#![feature(box_syntax)]
#![feature(new_uninit)]
#![feature(never_type)]
#![feature(const_impl_trait)]
#![feature(const_fn_fn_ptr_basics)]
#![feature(const_raw_ptr_to_usize_cast)]
#![feature(min_type_alias_impl_trait)]
#![feature(asm)]

extern crate alloc;
#[macro_use]
extern crate sophon;

use core::panic::PanicInfo;

use alloc::vec;
use fdt::Fdt;
use sophon::arch::{Arch, TargetArch};
use sophon::initfs::InitFS;
use sophon::kernel_tasks::user::UserTask;
use sophon::kernel_tasks::Idle;
use sophon::memory::kernel::{KernelHeapAllocator, KERNEL_HEAP};
use sophon::memory::physical::PHYSICAL_MEMORY;
use sophon::task::scheduler::{AbstractScheduler, SCHEDULER};
use sophon::BootInfo;
use sophon::{scheme, task::*};

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
        unsafe { sophon::log::BOOT_LOG.set_mmio_address(uart) }
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

    ipc::init();
    log!("[kernel: ipc initialized]");

    scheme::register_kernel_schemes();
    log!("[kernel: schemes initialized]");

    InitFS::deserialize(boot_info.init_fs);
    log!("[kernel: initfs initialized]");

    let task = Task::create_kernel_task(box Idle);
    log!("[kernel: created kernel process: {:?}]", task.id());

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
