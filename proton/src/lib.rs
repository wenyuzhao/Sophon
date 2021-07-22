#![allow(incomplete_features)]
#![feature(format_args_nl)]
#![feature(associated_type_defaults)]
#![feature(box_syntax)]
#![feature(core_intrinsics)]
#![feature(never_type)]
#![feature(const_fn_transmute)]
#![feature(const_raw_ptr_deref)]
#![feature(const_panic)]
#![feature(specialization)]
#![feature(const_mut_refs)]
#![feature(impl_trait_in_bindings)]
#![feature(min_type_alias_impl_trait)]
#![feature(step_trait)]
#![feature(global_asm)]
#![feature(asm)]
#![feature(const_impl_trait)]
#![feature(const_fn_fn_ptr_basics)]
#![feature(const_trait_impl)]
#![feature(const_generics)]
#![feature(const_maybe_uninit_assume_init)]
#![feature(allocator_api)]
#![feature(const_fn_trait_bound)]
#![feature(const_raw_ptr_to_usize_cast)]
#![feature(const_option)]
#![no_std]

use core::ops::Range;
use utils::{page::Frame, volatile::Volatile};

extern crate alloc;
extern crate elf_rs;

#[macro_use]
pub mod utils;
#[cfg(feature = "kernel")]
#[macro_use]
pub mod log;
#[cfg(feature = "kernel")]
pub mod arch;
#[cfg(feature = "kernel")]
pub mod boot_driver;
#[cfg(feature = "kernel")]
pub mod kernel_tasks;
#[cfg(feature = "kernel")]
pub mod memory;
pub mod task;
#[macro_use]
pub mod user;

pub struct BootInfo {
    pub available_physical_memory: &'static [Range<Frame>],
    pub device_tree: &'static [u8],
}

// const UART: usize = 0x9000000;
const UART: usize = 0xdead_0000_0000;

#[repr(C)]
pub struct GPIORegisters {
    pub gpfsel0: Volatile<u32>,   // 0x0
    pub gpfsel1: Volatile<u32>,   // 0x04
    pub gpfsel2: Volatile<u32>,   // 0x08
    pub gpfsel3: Volatile<u32>,   // 0x0c
    pub gpfsel4: Volatile<u32>,   // 0x10
    pub gpfsel5: Volatile<u32>,   // 0x14
    _0: [u8; 4],                  // 0x18
    pub gpset0: Volatile<u32>,    // 0x1c
    pub gpset1: Volatile<u32>,    // 0x20
    _1: [u8; 4],                  // 0x24
    pub gpclr0: Volatile<u32>,    // 0x28
    pub gpclr1: Volatile<u32>,    // 0x2c
    _2: [u8; 4],                  // 0x30
    pub gplev0: Volatile<u32>,    // 0x34
    pub gplev1: Volatile<u32>,    // 0x38
    _3: [u8; 4],                  // 0x3c
    pub gpeds0: Volatile<u32>,    // 0x40
    pub gpeds1: Volatile<u32>,    // 0x44
    _4: [u8; 4],                  // 0x48
    pub gpren0: Volatile<u32>,    // 0x4c
    pub gpren1: Volatile<u32>,    // 0x50
    _5: [u8; 4],                  // 0x54
    pub gpfen0: Volatile<u32>,    // 0x58
    pub gpfen1: Volatile<u32>,    // 0x5c
    _6: [u8; 4],                  // 0x60
    pub gphen0: Volatile<u32>,    // 0x64
    pub gphen1: Volatile<u32>,    // 0x68
    _7: [u8; 4],                  // 0x6c
    pub gplen0: Volatile<u32>,    // 0x70
    pub gplen1: Volatile<u32>,    // 0x74
    _8: [u8; 4],                  // 0x78
    pub gparen0: Volatile<u32>,   // 0x7c
    pub gparen1: Volatile<u32>,   // 0x80
    _9: [u8; 4],                  // 0x84
    pub gpafen0: Volatile<u32>,   // 0x88
    pub gpafen1: Volatile<u32>,   // 0x8c
    _10: [u8; 4],                 // 0x90
    pub gppud: Volatile<u32>,     // 0x94
    pub gppudclk0: Volatile<u32>, // 0x98
    pub gppudclk1: Volatile<u32>, // 0x9c
}

pub unsafe fn test_uart() {
    let gpio = &mut *(0xFE20_0000 as *mut GPIORegisters);
    gpio.gpfsel1.set((0b100 << 12) | (0b100 << 15));
    gpio.gppud.set(0);
    for _ in 0..150 {
        asm!("nop")
    }
    gpio.gppudclk0.set((1 << 14) | (1 << 15));
    for _ in 0..150 {
        asm!("nop")
    }
    gpio.gppudclk0.set(0);
    use utils::volatile::*;
    #[repr(C)]
    struct UARTRegisters {
        pub dr: Volatile<u32>,     // 0x00
        pub rsrecr: Volatile<u32>, // 0x04
        _0: [u8; 16],              // 0x08
        pub fr: Volatile<u32>,     // 0x18,
        _1: [u8; 4],               // 0x1c,
        pub ilpr: Volatile<u32>,   // 0x20,
        pub ibrd: Volatile<u32>,   // 0x24,
        pub fbrd: Volatile<u32>,   // 0x28,
        pub lcrh: Volatile<u32>,   // 0x2c,
        pub cr: Volatile<u32>,     // 0x30,
        pub ifls: Volatile<u32>,   // 0x34,
        pub imsc: Volatile<u32>,   // 0x38,
        pub ris: Volatile<u32>,    // 0x3c,
        pub mis: Volatile<u32>,    // 0x40,
        pub icr: Volatile<u32>,    // 0x44,
        pub dmacr: Volatile<u32>,  // 0x48,
    }
    let uart = &mut *(UART as *mut UARTRegisters);
    uart.cr.set(0);
    uart.icr.set(0);
    uart.ibrd.set(26);
    uart.fbrd.set(3);
    uart.lcrh.set((0b11 << 5) | (0b1 << 4));
    uart.cr.set((1 << 0) | (1 << 8) | (1 << 9));
    let mut putc = |c: char| {
        while uart.fr.get() & (1 << 5) != 0 {}
        uart.dr.set(c as u8 as u32);
    };
    let s = "#UART ENABLED\r\n";
    for c in s.chars() {
        putc(c)
    }
}

pub unsafe fn test_uart2() {
    // return;
    // asm!("msr daifclr, #2");
    use utils::volatile::*;
    #[repr(C)]
    struct UARTRegisters {
        pub dr: Volatile<u32>,     // 0x00
        pub rsrecr: Volatile<u32>, // 0x04
        _0: [u8; 16],              // 0x08
        pub fr: Volatile<u32>,     // 0x18,
        _1: [u8; 4],               // 0x1c,
        pub ilpr: Volatile<u32>,   // 0x20,
        pub ibrd: Volatile<u32>,   // 0x24,
        pub fbrd: Volatile<u32>,   // 0x28,
        pub lcrh: Volatile<u32>,   // 0x2c,
        pub cr: Volatile<u32>,     // 0x30,
        pub ifls: Volatile<u32>,   // 0x34,
        pub imsc: Volatile<u32>,   // 0x38,
        pub ris: Volatile<u32>,    // 0x3c,
        pub mis: Volatile<u32>,    // 0x40,
        pub icr: Volatile<u32>,    // 0x44,
        pub dmacr: Volatile<u32>,  // 0x48,
    }
    let uart = &mut *(UART as *mut UARTRegisters);
    let mut putc = |c: char| {
        while uart.fr.get() & (1 << 5) != 0 {}
        uart.dr.set(c as u8 as u32);
    };
    let s = "#UART ENABLED 2\r\n";
    putc('F');
    putc('U');
    putc('C');
    putc('K');
    putc('\r');
    putc('\n');
}
