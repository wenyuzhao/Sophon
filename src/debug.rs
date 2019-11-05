use core::fmt;
use core::fmt::Write;
use spin::Mutex;
use core::intrinsics::volatile_load;
use core::intrinsics::volatile_store;
use crate::gpio::*;


#[doc(hidden)]
pub fn _println(args: fmt::Arguments) {
    let mut write = UART.lock();
    write.write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => ({
        $crate::debug::_println(format_args_nl!($($arg)*))
    });
}

pub fn print_boot(s: &str) {
    const UART_DR: *mut u32 = unsafe { (UART0::UART_DR as usize & 0x0000ffff_ffffffff) as _ };
    const UART_FR: *mut u32 = unsafe { (UART0::UART_FR as usize & 0x0000ffff_ffffffff) as _ };
    for b in s.bytes() {
        while (unsafe { *UART_FR }) & (1 << 5) > 0 {}
        unsafe { *UART_DR = b as u32 };
    }
    while (unsafe { *UART_FR }) & (1 << 5) > 0 {}
    unsafe { *UART_DR = '\n' as u32 };
}

pub static UART: Mutex<UART0> = Mutex::new(UART0);

pub struct UART0;

impl UART0 {
    const UART_DR: *mut u32 = (PERIPHERAL_BASE + 0x201000) as _;
    const UART_FR: *mut u32 = (PERIPHERAL_BASE + 0x201018) as _;

    fn mmio_write(&self, reg: *mut u32, val: u32) {
        unsafe { volatile_store(reg as *mut u32, val) }
    }
    
    fn mmio_read(&self, reg: *mut u32) -> u32 {
        unsafe { volatile_load(reg as *const u32) }
    }
    
    fn transmit_fifo_full(&self) -> bool {
        self.mmio_read(Self::UART_FR) & (1 << 5) > 0
    }
    
    fn receive_fifo_empty(&self) -> bool {
        self.mmio_read(Self::UART_FR) & (1 << 4) > 0
    }
    
    fn putc(&self, c: u8) {
        while self.transmit_fifo_full() {}
        self.mmio_write(Self::UART_DR, c as u32);
    }
    
    fn getc(&self) -> u8 {
        while self.receive_fifo_empty() {}
        self.mmio_read(Self::UART_DR) as u8
    }
}

impl Write for UART0 {
    fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
        for b in s.bytes() {
            self.putc(b)
        }
        Ok(())
    }
}



pub struct GPIO18;

impl GPIO18 {
    const GPFSEL1: *mut u32 = (GPIO_BASE + 0x4) as _;
    const GPSET0: *mut u32 = (GPIO_BASE + 0x1c) as _;
    const GPCLR0: *mut u32 = (GPIO_BASE + 0x28) as _;

    pub fn init() {
        // 1. Set GPIO Pin 18 is an output 
        unsafe {
            let mut v = volatile_load(Self::GPFSEL1);
            v &= !(0b111 << 24);
            v |= 0b001 << 24;
            volatile_store(Self::GPFSEL1, v);
        }
    }

    pub fn set(v: bool) {
        unsafe {
            if v {
                volatile_store(Self::GPSET0, 1 << 18);
            } else {
                volatile_store(Self::GPCLR0, 1 << 18);
            }
        }
    }
}
