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
mod random;
mod exception;
mod start;
mod mm;
mod interrupt;
mod timer;
mod task;
mod init_process;
mod kernel_process;
mod utils;
mod gic;
use cortex_a::regs::*;

#[global_allocator]
static ALLOCATOR: mm::heap::GlobalAllocator = mm::heap::GlobalAllocator::new();


pub extern fn kmain() -> ! {
    
    // crate::debug_boot::UART::init();
    // crate::debug_boot::UART::init();
    
    // unsafe {  asm!("msr cpacr_el1, $0"::"r"(0xfffffff)); }
    
    // debug_boot::PL011UartInne();
    // let el = (CurrentEL.get() & 0b1100) >> 2;
    // loop {}
    println!("Hello, Raspberry PI!");
    // boot_log!("Current execution level: {}", (CurrentEL.get() & 0b1100) >> 2);
    // loop {}
    ALLOCATOR.init();
    println!("Hello");
    // debug_assert!(false);
    // println!("Random: {} {} {}", random::random(0, 100), random::random(0, 100), random::random(0, 100));
    // loop {}
    println!("Current execution level: {}", (CurrentEL.get() & 0b1100) >> 2);
    // // Initialize & start timer
    interrupt::initialize();
    timer::init();
    println!("Timer init");
    // println!("{}", interrupt::is_enabled());
    interrupt::enable();
    println!("Int init");
    println!("{}", interrupt::is_enabled());
    // unsafe { *(0xdeadbeef as *mut u8) = 0; }
    // let mut i = 0;
    // loop {
        //                            0b10000
        // 0b1101000000
        // 0b1101000000
        // if timer::timer_count() < 7000000 {
            // println!("------------------------------- {}", i);
            // i += 1;
            // println!("SPSR_EL1 = 0b{:b}", DAIF.get());
            // println!("IRQ_PENDING_1 = 0b{:b}", unsafe { *interrupt::IRQ_PENDING_1 });
            // println!("IRQ_PENDING_2 = 0b{:b}", unsafe { *interrupt::IRQ_PENDING_2 });
            // println!("IRQ_BASIC_PENDING = 0b{:b}", unsafe { *interrupt::IRQ_BASIC_PENDING });
            // println!("TIMER_CS = 0b{:b}", unsafe { *timer::TIMER_CS });
            // println!("Current execution level: {}", (CurrentEL.get() & 0b1100) >> 2);
            // println!("ENABLE_IRQS_1 = 0b{:b}", unsafe { *interrupt::ENABLE_IRQS_1 });
            // println!("ENABLE_IRQS_2 = 0b{:b}", unsafe { *interrupt::ENABLE_IRQS_2 });
            // println!("ENABLE_BASIC_IRQS = 0b{:b}", unsafe { *interrupt::ENABLE_BASIC_IRQS });
            // println!("ARMTIMER_VALUE = {}", unsafe { *timer::ARMTIMER_VALUE });
            // println!("GICC_IAR = {}", unsafe { *timer::GICC_IAR });
            // println!("nIRQ = {}", unsafe { (*timer::GICC_IAR) & timer::GICC_IAR_INTERRUPT_ID__MASK });
    //         u32 nIAR = read32 (GICC_IAR);

	// unsigned nIRQ = nIAR & GICC_IAR_INTERRUPT_ID__MASK;
            // if (unsafe { *timer::TIMER_CS }) != 0 {
            //     unsafe { *timer::TIMER_CS = 0 };
            // }
            // unsafe { asm!("svc #0"); }
            // for i in 0..10000000 {}

            // unsafe { *timer::GICD_SGIR =  1 << (0 + timer::GICD_SGIR_CPU_TARGET_LIST__SHIFT) | 1; }
        // }
    // }

    let task = task::Task::create_kernel_task(kernel_process::main);
    println!("Created kernel process: {:?} {:?}", task.id(), task.context.pc);
    let task = task::Task::create_kernel_task(init_process::entry);
    println!("Created init process: {:?}", task.id());
    let task = task::Task::create_kernel_task(kernel_process::idle);
    println!("Created idle process: {:?}", task.id());

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