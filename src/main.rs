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
mod gpio;
#[macro_use]
mod debug;
#[macro_use]
mod syscall;
mod mailbox;
mod fb;
mod random;
mod exception;
mod start;
mod mm;
mod interrupt;
mod timer;
mod task;
mod init;
use cortex_a::regs::*;

#[global_allocator]
static ALLOCATOR: mm::heap::GlobalAllocator = mm::heap::GlobalAllocator::new();

use core::sync::atomic::{AtomicUsize, Ordering};
static ID: AtomicUsize = AtomicUsize::new(0);

extern fn init_process() -> ! {
    let id = ID.fetch_add(1, Ordering::SeqCst);
    debug!("Start init {:?}", task::Task::current().unwrap().id());
    task::exec::exec_user(init::INIT_ELF);
    unreachable!();
}



pub fn kmain() -> ! {
    debug!("Hello, Raspberry PI!");
    ALLOCATOR.init();
    {
        // // Test allocator
        let v = vec![1, 1, 2, 3, 5, 7];
        let b = box 233;
        debug!("Heap allocation: {:?}, {}", v, b);
    }
    {
        let mut fb = fb::FRAME_BUFFER.lock();
        fb.init();
        fb.clear(fb::Color::rgba(0x0000FFFF));
    }
    debug!("Random: {} {} {}", random::random(0, 100), random::random(0, 100), random::random(0, 100));
    debug!("Current execution level: {}", (CurrentEL.get() & 0b1100) >> 2);
    // Initialize & start timer
    timer::init();
    interrupt::enable();

    let task = task::Task::create_init_task(init_process);
    debug!("Created init process: {:?}", task.id());

    // Manually trigger a page fault
    // unsafe { *(0xdeadbeef as *mut u8) = 0; }
    loop {
        task::GLOBAL_TASK_SCHEDULER.schedule();
    }
}



#[cfg(not(feature="rls"))]
#[panic_handler]
fn panic(info: &::core::panic::PanicInfo) -> ! {
    debug!("{}", info);
    loop {}
}

#[cfg(not(feature="rls"))]
#[alloc_error_handler]
fn alloc_error_handler(layout: ::alloc::alloc::Layout) -> ! {
    panic!("Allocation error: {:?}", layout)
}