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
pub mod kernel_tasks;
pub mod memory;
pub mod modules;
pub mod task;

use core::panic::PanicInfo;

use crate::arch::{Arch, TargetArch};
use crate::kernel_tasks::Idle;
use crate::memory::kernel::{KernelHeapAllocator, KERNEL_HEAP};
use crate::memory::physical::PHYSICAL_MEMORY;
use crate::task::scheduler::{AbstractScheduler, SCHEDULER};
use crate::task::Proc;
use alloc::boxed::Box;
use boot::BootInfo;
use fdt::Fdt;
use vfs::ramfs::RamFS;

#[global_allocator]
static ALLOCATOR: KernelHeapAllocator = KernelHeapAllocator;

fn display_banner() {
    println!(r"");
    println!(r" ____ ____ ___  _  _ ____ _  _    ____ ____ ");
    println!(r" [__  |  | |__] |__| |  | |\ |    |  | [__  ");
    println!(
        r" ___] |__| |    |  | |__| | \|    |__| ___]   v{}",
        env!("CARGO_PKG_VERSION")
    );
    println!(r"");
    println!(r" Hello Sophon! ");
    println!(r"");
}

#[no_mangle]
pub extern "C" fn _start(boot_info: &'static BootInfo) -> isize {
    if let Some(uart) = boot_info.uart {
        utils::boot_log::init(uart);
    }
    log!("boot_info @ {:?} {:?}", boot_info as *const _, unsafe {
        *(boot_info as *const _ as *const usize)
    });
    log!("device_tree @ {:?}", boot_info.device_tree.as_ptr_range());
    log!(
        "available_physical_memory @ {:?}",
        boot_info.available_physical_memory.as_ptr_range()
    );

    display_banner();

    // Initialize physical memory and kernel heap
    log!("[kernel] initialize physical memory");
    PHYSICAL_MEMORY.init(boot_info.available_physical_memory);
    log!("[kernel] initialize kernel heap");
    KERNEL_HEAP.init();

    // Initialize arch and boot drivers
    log!("[kernel] load fdt");
    let fdt = Fdt::new(boot_info.device_tree).unwrap();
    log!("[kernel] arch-specific initialization");
    TargetArch::init(fdt);

    log!("[kernel] initialize syscall");
    task::syscall::init();

    log!("[kernel] load init-fs");
    let initfs = Box::leak(box RamFS::deserialize(boot_info.init_fs));

    let load_module_from_initfs = |name: &str, path: &str| {
        log!("[kernel]  - load module '{}'", name);
        let file = initfs.get(path).unwrap().as_file().unwrap();
        crate::modules::register(name, file.to_vec());
    };
    log!("[kernel] load kernel modules...");
    load_module_from_initfs("hello", "/etc/modules/libhello.so");
    load_module_from_initfs("vfs", "/etc/modules/libvfs.so");
    crate::modules::init_vfs(initfs);
    load_module_from_initfs("dev", "/etc/modules/libdev.so");
    load_module_from_initfs("pl011", "/etc/modules/libpl011.so");
    log!("[kernel] kernel modules loaded");

    log!("[kernel] start idle process");
    let _proc = Proc::spawn(box Idle);

    log!("[kernel] start init process");
    let init = initfs.get("/bin/init").unwrap().as_file().unwrap().to_vec();
    let _proc = Proc::spawn_user(init.to_vec());

    log!("[kernel] start tty process");
    let tty = initfs.get("/bin/tty").unwrap().as_file().unwrap().to_vec();
    let _proc = Proc::spawn_user(tty.to_vec());

    log!("[kernel] start timer");
    TargetArch::interrupt().start_timer();

    log!("[kernel] start scheduler");
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
