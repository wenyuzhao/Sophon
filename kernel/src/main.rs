#![feature(asm)]
#![feature(format_args_nl)]
#![feature(global_asm)]
#![feature(panic_info_message)]
#![feature(core_intrinsics)]
#![feature(stmt_expr_attributes)]
#![feature(naked_functions)]
#![feature(const_fn)]
#![feature(const_raw_ptr_to_usize_cast)]
#![feature(step_trait)]
#![feature(const_transmute)]
#![feature(box_syntax)]
#![feature(alloc_error_handler)]
#![feature(new_uninit)]
#![feature(type_alias_impl_trait)]
#![feature(internal_uninit_const)]
#![feature(never_type)]
#![allow(dead_code)]
#![no_std]
#![no_main]

#[macro_use]
extern crate lazy_static;
extern crate spin;
extern crate cortex_a;
extern crate proton;
#[macro_use]
extern crate bitflags;
#[allow(unused)]
#[macro_use]
extern crate alloc;
extern crate goblin;
extern crate device_tree;
#[macro_use]
mod utils;
#[macro_use]
mod debug;
mod ipc;
mod memory;
mod heap;
mod task;
mod init_process;
mod kernel_process;
mod arch;
mod drivers;
use cortex_a::regs::*;
use arch::*;

#[global_allocator]
static ALLOCATOR: heap::GlobalAllocator = heap::GlobalAllocator::new();

// static DEVICE_TREE: &'static [u8] = include_bytes!("../bcm2711-rpi-4-b.dtb");
static DEVICE_TREE: &'static [u8] = include_bytes!("../bcm2710-rpi-3-b.dtb");

pub extern fn kmain() -> ! {
    println!("Hello, Raspberry PI!");
    println!("[kernel: current execution level = {}]", (CurrentEL.get() & 0b1100) >> 2);
    // Initialize kernel heap
    ALLOCATOR.init();
    // println!("Random: {} {} {}", random::random(0, 100), random::random(0, 100), random::random(0, 100));
    // Initialize & start timer
    Target::Interrupt::init();
    println!("[kernel: interrupt initialized]");
    ipc::init();
    println!("[kernel: ipc initialized]");
    Target::Timer::init();
    println!("[kernel: timer initialized]");

    // let dt = device_tree::DeviceTree::load(DEVICE_TREE).unwrap();
    // println!("{:#?}", dt);

    // drivers::emmc::EMMC::init().unwrap();
    // drivers::fat::FAT::init().unwrap();
    // drivers::fat::FAT::ls_root();

    // println!("FINISH"); loop {}

    let task = task::Task::create_kernel_task(kernel_process::main);
    println!("Created kernel process: {:?}", task.id());
    let task = task::Task::create_kernel_task(kernel_process::idle);
    println!("Created idle process: {:?}", task.id());
    let task = task::Task::create_kernel_task(init_process::entry);
    println!("Created init process: {:?}", task.id());
    
    // Manually trigger a page fault
    // unsafe { *(0xdeadbeef as *mut u8) = 0; }
    
    task::GLOBAL_TASK_SCHEDULER.schedule();
}



#[panic_handler]
fn panic(info: &::core::panic::PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

#[alloc_error_handler]
fn alloc_error_handler(layout: ::alloc::alloc::Layout) -> ! {
    panic!("Allocation error: {:?}", layout)
}