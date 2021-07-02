use core::{
    fmt::{self, Write},
    mem, slice,
};

use crate::{boot_driver::BootDriver, utils::volatile::Volatile};
use device_tree::Node;
use spin::{Lazy, Mutex};

#[repr(C)]
pub struct UARTRegisters {
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

pub struct UART0 {
    uart: Option<*mut UARTRegisters>,
}

unsafe impl Send for UART0 {}
unsafe impl Sync for UART0 {}

impl UART0 {
    fn transmit_fifo_full(&self) -> bool {
        self.uart().fr.get() & (1 << 5) > 0
    }

    fn receive_fifo_empty(&self) -> bool {
        self.uart().fr.get() & (1 << 4) > 0
    }

    fn uart(&self) -> &mut UARTRegisters {
        unsafe { &mut *self.uart.unwrap() }
    }

    pub fn putchar(&self, c: char) {
        while self.transmit_fifo_full() {}
        self.uart().dr.set(c as _);
    }

    #[inline(never)]
    pub fn init_uart(&self) {
        let uart = self.uart();
        // let gpio = GPIORegisters::get_low();

        uart.cr.set(0);
        uart.icr.set(0);
        uart.ibrd.set(26);
        uart.fbrd.set(3);
        uart.lcrh.set((0b11 << 5) | (0b1 << 4));
        uart.cr.set((1 << 0) | (1 << 8) | (1 << 9));

        // gpio.gpfsel1.set((0b100 << 12) | (0b100 << 15));
        // gpio.gppud.set(0);
        // wait_cycles(150);
        // gpio.gppudclk0.set((1 << 14) | (1 << 15));
        // wait_cycles(150);
        // gpio.gppudclk0.set(0);
    }
}

fn wait_cycles(n: usize) {
    for _ in 0..n {
        unsafe {
            llvm_asm!("nop");
        }
    }
}

pub static UART: Lazy<Mutex<UART0>> = Lazy::new(|| Mutex::new(UART0 { uart: None }));

impl BootDriver for UART0 {
    const COMPATIBLE: &'static str = "arm,pl011\0arm,primecell";
    fn init(&mut self, node: &Node) {
        let reg = node.prop_raw("reg").unwrap();
        let len = reg.len() / 4;
        let data = unsafe { slice::from_raw_parts(reg.as_ptr() as *const u32, len) };
        let uart_address = ((u32::from_be(data[0]) as u64) << 32) | (u32::from_be(data[1]) as u64);
        self.uart = Some(unsafe { mem::transmute(uart_address) });
        self.init_uart();
        *crate::log::WRITER.lock() = Some(box Log);
    }
}

pub struct Log;

impl Write for Log {
    fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
        let uart = UART.lock();
        for c in s.chars() {
            uart.putchar(c);
        }
        Ok(())
    }
}
