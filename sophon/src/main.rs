#![allow(incomplete_features)]
#![feature(format_args_nl)]
#![feature(step_trait)]
#![feature(const_trait_impl)]
#![feature(alloc_error_handler)]
#![feature(adt_const_params)]
#![feature(generic_const_exprs)]
#![feature(type_alias_impl_trait)]
#![feature(downcast_unchecked)]
#![feature(const_option_ext)]
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
pub mod task;

use core::panic::PanicInfo;

use crate::arch::{Arch, TargetArch};
use crate::memory::kernel::{KernelHeapAllocator, KERNEL_HEAP};
use crate::memory::physical::PHYSICAL_MEMORY;
use crate::task::proc::PROCESS_MANAGER;
use alloc::boxed::Box;
use boot::BootInfo;
use device_tree::DeviceTree;
use spin::{Mutex, Once};
use task::sched::SCHEDULER;
use vfs::ramfs::RamFS;

#[global_allocator]
static ALLOCATOR: KernelHeapAllocator = KernelHeapAllocator;

static mut DEV_TREE: Option<DeviceTree<'static, 'static>> = None;
static INIT_FS: Once<&'static RamFS> = Once::new();

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
pub extern "C" fn _start(boot_info: &'static BootInfo) -> isize {
    if let Some(uart) = boot_info.uart {
        utils::boot_logger::init(uart);
    }
    info!("boot_info @ {:?} {:?}", boot_info as *const _, unsafe {
        *(boot_info as *const _ as *const usize)
    });
    info!("device_tree @ {:?}", boot_info.device_tree.as_ptr_range());
    info!(
        "available_physical_memory @ {:?}",
        boot_info.available_physical_memory.as_ptr_range()
    );

    display_banner();

    // Initialize physical memory and kernel heap
    info!("initialize physical memory");
    PHYSICAL_MEMORY.init(boot_info.available_physical_memory);
    info!("initialize kernel heap");
    KERNEL_HEAP.init();

    // Initialize arch and boot drivers
    info!("load device tree");
    unsafe {
        DEV_TREE = DeviceTree::new(boot_info.device_tree);
    }
    info!("arch-specific initialization");
    TargetArch::init(boot_info);

    info!("load init-fs");
    INIT_FS.call_once(|| Box::leak(Box::new(RamFS::deserialize(boot_info.init_fs))));
    let initfs = *INIT_FS.get().unwrap();

    info!("load kernel modules...");
    for (name, path) in ALL_MODULES {
        info!(" - load module '{}'", name);
        let file = initfs.get(path).unwrap().as_file().unwrap();
        crate::modules::register(name, file.to_vec());
    }
    info!("kernel modules loaded");

    info!("start sched process (pid=0)");
    PROCESS_MANAGER.spawn_sched_process();

    info!("start init process (pid=1)");
    let _proc = PROCESS_MANAGER.spawn_init_process();

    if cfg!(sophon_test) {
        info!("run boot tests");
        crate::utils::testing::run_boot_tests();
    }

    info!("start scheduler");
    SCHEDULER.schedule();
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    error!("{}", info);
    static DOUBLE_PANIC: Mutex<bool> = Mutex::new(false);
    let mut double_panic = DOUBLE_PANIC.lock();
    if !*double_panic {
        *double_panic = true;
        TargetArch::halt(-1)
    } else {
        drop(double_panic);
        info!("ERROR: Double panic!");
        loop {}
    }
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
