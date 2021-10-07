use crate::{boot_driver::BootDriver, utils::volatile::Volatile};
use core::fmt::{self, Write};
use fdt::node::FdtNode;
use memory::{
    address::{Address, P},
    page::Frame,
};

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
    const fn new() -> Self {
        Self { uart: None }
    }

    fn transmit_fifo_full(&self) -> bool {
        self.uart().fr.get() & (1 << 5) != 0
    }

    // fn receive_fifo_empty(&self) -> bool {
    //     self.uart().fr.get() & (1 << 4) != 0
    // }

    fn uart(&self) -> &mut UARTRegisters {
        unsafe { &mut *self.uart.unwrap() }
    }

    fn putchar(&self, c: char) {
        if self.uart.is_none() {
            return;
        }
        while self.transmit_fifo_full() {}
        self.uart().dr.set(c as u8 as u32);
    }

    fn init_uart(&self) {
        let uart = self.uart();
        uart.cr.set(0);
        uart.icr.set(0);
        uart.ibrd.set(26);
        uart.fbrd.set(3);
        uart.lcrh.set((0b11 << 5) | (0b1 << 4));
        uart.cr.set((1 << 0) | (1 << 8) | (1 << 9));
    }
}

pub static mut UART: UART0 = UART0::new();

impl BootDriver for UART0 {
    const COMPATIBLE: &'static [&'static str] = &["arm,pl011"];
    fn init(&mut self, node: &FdtNode, parent: Option<&FdtNode>) {
        let mut uart_frame =
            Address::<P>::new(node.reg().unwrap().next().unwrap().starting_address as usize);
        uart_frame = Self::translate_address(uart_frame, parent.unwrap());
        let uart_page = Self::map_device_page(Frame::new(uart_frame));
        self.uart = Some(uart_page.start().as_mut_ptr());
        self.init_uart();
        *crate::log::WRITER.lock() = Some(box Log);
        log!("UART @ {:?} -> {:?}", uart_frame, uart_page);
    }
}

pub struct Log;

impl Write for Log {
    fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
        for c in s.chars() {
            if c == '\n' {
                unsafe {
                    UART.putchar('\r');
                }
            }
            unsafe {
                UART.putchar(c);
            }
        }
        Ok(())
    }
}
