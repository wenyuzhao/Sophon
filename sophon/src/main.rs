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
extern crate sophon_macros;

#[macro_use]
pub mod utils;
pub mod arch;
pub mod memory;
pub mod modules;
pub mod scheduler;
pub mod task;

use core::panic::PanicInfo;

use crate::arch::{Arch, TargetArch};
use crate::memory::kernel::{KernelHeapAllocator, KERNEL_HEAP};
use crate::memory::physical::PHYSICAL_MEMORY;
use crate::scheduler::{AbstractScheduler, SCHEDULER};
use crate::task::runnable::Idle;
use crate::task::Proc;
use alloc::boxed::Box;
use boot::BootInfo;
use device_tree::DeviceTree;
use vfs::ramfs::RamFS;

#[global_allocator]
static ALLOCATOR: KernelHeapAllocator = KernelHeapAllocator;

static mut DEV_TREE: Option<DeviceTree<'static, 'static>> = None;

fn display_banner() {
    let ver = env!("CARGO_PKG_VERSION");
    println!(r"");
    println!(r" ____ ____ ___  _  _ ____ _  _    ____ ____ ");
    println!(r" [__  |  | |__] |__| |  | |\ |    |  | [__  ");
    println!(r" ___] |__| |    |  | |__| | \|    |__| ___]   v{}", ver);
    println!(r"");
    println!(r" Hello Sophon! ");
    println!(r"");
}

const ALL_MODULES: &'static [(&'static str, &'static str)] = &[
    ("hello", "/etc/modules/libhello.so"),
    ("bcm2711-gpio", "/etc/modules/libbcm2711_gpio.so"),
    ("gic", "/etc/modules/libgic.so"),
    ("gic-timer", "/etc/modules/libgic_timer.so"),
    ("vfs", "/etc/modules/libvfs.so"),
    ("dev", "/etc/modules/libdev.so"),
    ("pl011", "/etc/modules/libpl011.so"),
];

#[no_mangle]
pub extern "C" fn _start(boot_info: &'static BootInfo, core: usize) -> isize {
    if core != 0 {
        _start_ap(core);
    }
    if let Some(uart) = boot_info.uart {
        utils::boot_logger::init(uart);
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
    log!("[kernel] load device tree");
    unsafe {
        DEV_TREE = DeviceTree::new(boot_info.device_tree);
    }
    log!("[kernel] arch-specific initialization");
    TargetArch::init(boot_info);

    log!("[kernel] load init-fs");
    let initfs = Box::leak(box RamFS::deserialize(boot_info.init_fs));
    let initfs_ptr = initfs as *mut _;

    log!("[kernel] load kernel modules...");
    for (name, path) in ALL_MODULES {
        log!("[kernel]  - load module '{}'", name);
        let file = initfs.get(path).unwrap().as_file().unwrap();
        crate::modules::register(name, file.to_vec());
        if *name == "vfs" {
            crate::modules::init_vfs(initfs_ptr);
        }
    }
    log!("[kernel] kernel modules loaded");

    log!("[kernel] start idle process");
    let _proc = Proc::spawn(box Idle);

    log!("[kernel] start init process");
    let init = initfs.get("/bin/init").unwrap().as_file().unwrap().to_vec();
    let _proc = Proc::spawn_user(init.to_vec(), &[]);

    if cfg!(sophon_test) {
        log!("[kernel] start boot tests");
        crate::utils::testing::run_boot_tests();
        log!("[kernel] start kernel test runner");
        crate::utils::testing::start_kernel_test_runner();
    }

    if let Some(start_ap) = boot_info.start_ap {
        log!("[kernel] start ap");
        start_ap();
    }

    log!("[kernel] start scheduler");
    SCHEDULER.schedule();
}

fn _start_ap(core: usize) -> ! {
    log!("AP #{}", core);
    // TODO: Start scheduling
    loop {}
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    log!("{}", info);
    TargetArch::halt(-1)
}

#[no_mangle]
pub extern "C" fn __chkstk() {}

#[alloc_error_handler]
fn alloc_error_handler(layout: ::alloc::alloc::Layout) -> ! {
    panic!("Allocation error: {:?}", layout)
}

#[test(boot)]
fn boot_alloc_test() {
    let mut array = alloc::vec![0usize; 0];
    for v in 1..=100 {
        array.push(v);
    }
    let sum: usize = array.iter().sum();
    assert_eq!(sum, (1 + 100) * 100 / 2);
}
