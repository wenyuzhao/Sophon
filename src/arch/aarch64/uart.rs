use core::fmt;
use core::fmt::Write;
use spin::Mutex;
use core::intrinsics::volatile_load;
use core::intrinsics::volatile_store;
use super::constants::*;
use crate::arch::*;

pub fn boot_time_log(s: &str) {
    fn putc(c: char) {
        unsafe { 
            while (*UART0::LOW_UART_FR) & (1 << 5) > 0 {}
            *UART0::LOW_UART_DR = c as u32;
        }
    }
    for c in s.chars() {
        if c == '\n' {
            putc('\r')
        }
        putc(c)
    }
    putc('\r');
    putc('\n');
}

pub struct UART0;

impl UART0 {
    const UART_DR: *mut u32 = (PERIPHERAL_BASE + 0x201000) as _;
    const UART_FR: *mut u32 = (PERIPHERAL_BASE + 0x201018) as _;

    const LOW_BASE: usize = (PERIPHERAL_BASE + 0x201000) & 0xffffffffffff;
    const LOW_UART_DR: *mut u32   = (Self::LOW_BASE + 0x00) as _;
    const LOW_UART_FR: *mut u32   = (Self::LOW_BASE + 0x18) as _;
    const LOW_UART_IBRD: *mut u32 = (Self::LOW_BASE + 0x24) as _;
    const LOW_UART_FBRD: *mut u32 = (Self::LOW_BASE + 0x28) as _;
    const LOW_UART_LCRH: *mut u32 = (Self::LOW_BASE + 0x2C) as _;
    const LOW_UART_CR: *mut u32   = (Self::LOW_BASE + 0x30) as _;
    const LOW_UART_ICR: *mut u32  = (Self::LOW_BASE + 0x44) as _;
    const LOW_GPIO_BASE: usize = (PERIPHERAL_BASE + 0x200000) & 0xffffffffffff;
    const LOW_GPFSEL1: *mut u32 = (Self::LOW_GPIO_BASE + 0x4) as _;
    const LOW_GPPUD: *mut u32 = (Self::LOW_GPIO_BASE + 0x94) as _;
    const LOW_GPPUDCLK0: *mut u32 = (Self::LOW_GPIO_BASE + 0x98) as _;
    const LOW_GPPUDCLK1: *mut u32 = (Self::LOW_GPIO_BASE + 0x9C) as _;

    fn mmio_write(reg: *mut u32, val: u32) {
        unsafe { volatile_store(reg as *mut u32, val) }
    }
    
    fn mmio_read(reg: *mut u32) -> u32 {
        unsafe { volatile_load(reg as *const u32) }
    }
    
    fn transmit_fifo_full() -> bool {
        Self::mmio_read(Self::UART_FR) & (1 << 5) > 0
    }
    
    fn receive_fifo_empty() -> bool {
        Self::mmio_read(Self::UART_FR) & (1 << 4) > 0
    }

    pub fn init() {
        unsafe {
            *Self::LOW_UART_CR = 0;
            *Self::LOW_UART_ICR = 0;
            *Self::LOW_UART_IBRD = 26;
            *Self::LOW_UART_FBRD = 3;
            *Self::LOW_UART_LCRH = (0b11 << 5) | (0b1 << 4);
            *Self::LOW_UART_CR = (1 << 0) | (1 << 8) | (1 << 9);
            
            *Self::LOW_GPFSEL1 = (0b100 << 12) | (0b100 << 15);
            *Self::LOW_GPPUD = 0;
            wait_cycles(150);
            *Self::LOW_GPPUDCLK0 = (1 << 14) | (1 << 15);
            wait_cycles(150);
            *Self::LOW_GPPUDCLK0 = 0;
        }
    }
}

impl AbstractLogger for UART0 {

    fn put(c: char) {
        while Self::transmit_fifo_full() {}
        Self::mmio_write(Self::UART_DR, c as u32);
    }
    
    // fn get(&self) -> char {
    //     while self.receive_fifo_empty() {}
    //     self.mmio_read(Self::UART_DR) as _
    // }
}

fn wait_cycles(n: usize) {
    for _ in 0..n {
        unsafe { asm!("nop"); }
    }
}
