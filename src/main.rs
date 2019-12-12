#![feature(asm)]
#![feature(format_args_nl)]
#![feature(global_asm)]
#![feature(panic_info_message)]
#![feature(core_intrinsics)]
#![feature(stmt_expr_attributes)]
#![feature(naked_functions)]
#![feature(const_fn)]
#![feature(const_raw_ptr_to_usize_cast)]
#![feature(never_type)]
#![feature(step_trait)]
#![feature(const_transmute)]
#![feature(box_syntax)]
#![feature(alloc_error_handler)]
#![feature(new_uninit)]
#![feature(type_alias_impl_trait)]
#![allow(unused)]
#![no_std]
#![no_main]

#[macro_use]
extern crate lazy_static;
extern crate spin;
extern crate cortex_a;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate alloc;
extern crate goblin;
#[macro_use]
mod debug_boot;
#[macro_use]
mod debug;
mod gpio;
#[macro_use]
mod syscall;
mod mailbox;
mod fb;
// mod random;
// mod exception;
// mod start;
mod mm;
// mod interrupt;
mod timer;
mod task;
mod init_process;
mod kernel_process;
mod utils;
mod gic;
mod arch;
use cortex_a::regs::*;
use arch::*;
#[global_allocator]
static ALLOCATOR: mm::heap::GlobalAllocator = mm::heap::GlobalAllocator::new();


pub extern fn kmain() -> ! {
    println!("Hello, Raspberry PI!");
    println!("Current execution level: {}", (CurrentEL.get() & 0b1100) >> 2);
    ALLOCATOR.init();
    Target::Interrupt::initialize();

    // println!("Random: {} {} {}", random::random(0, 100), random::random(0, 100), random::random(0, 100));
    // // Initialize & start timer
    // interrupt::initialize();
    syscall::init();
    timer::init();
    println!("Timer init");
    Target::Interrupt::enable();
    println!("Int init");

    let task = task::Task::create_kernel_task(kernel_process::main);
    println!("Created kernel process: {:?} {:?}", task.id(), task.context.pc);
    let task = task::Task::create_kernel_task(init_process::entry);
    println!("Created init process: {:?}", task.id());
    let task = task::Task::create_kernel_task(kernel_process::idle);
    println!("Created idle process: {:?}", task.id());
// loop {}

    // // Manually trigger a page fault
    // // unsafe { *(0xdeadbeef as *mut u8) = 0; }
    loop {
        task::GLOBAL_TASK_SCHEDULER.schedule();
    }
}



#[cfg(not(feature="rls"))]
#[panic_handler]
fn panic(info: &::core::panic::PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}

#[cfg(not(feature="rls"))]
#[alloc_error_handler]
fn alloc_error_handler(layout: ::alloc::alloc::Layout) -> ! {
    panic!("Allocation error: {:?}", layout)
}