#![feature(format_args_nl)]
#![feature(default_alloc_error_handler)]
#![no_std]

#[macro_use]
extern crate log;
extern crate alloc;

use dev::{DevRequest, Device};
use spin::RwLock;

use kernel_module::{kernel_module, KernelModule, SERVICE};
use memory::{page::Frame, volatile::Volatile};

#[kernel_module]
pub static PL011_MODULE: PL011 = PL011 {
    uart: RwLock::new(core::ptr::null_mut()),
};

unsafe impl Send for PL011 {}
unsafe impl Sync for PL011 {}

pub struct PL011 {
    pub uart: RwLock<*mut UART0>,
}

impl PL011 {
    fn uart(&self) -> &mut UART0 {
        unsafe { &mut **self.uart.read() }
    }
}

impl KernelModule for PL011 {
    fn init(&'static self) -> anyhow::Result<()> {
        log!("Hello, PL011!");
        let devtree = SERVICE.get_device_tree().unwrap();
        let node = devtree.compatible("arm,pl011").unwrap();
        let uart_frame = node.translate(node.regs().unwrap().next().unwrap().start);
        let uart_page = SERVICE.map_device_page(Frame::new(uart_frame));
        let uart = unsafe { &mut *(uart_page.start().as_mut_ptr() as *mut UART0) };
        uart.init();
        *self.uart.write() = uart;
        // let mut irqs = node.interrupts().unwrap();
        // let is_spi = irqs.next().unwrap() != 0;
        // let irq_base = if is_spi { 32 } else { 16 };
        // let irq_num = irqs.next().unwrap() + irq_base;

        log!("register_device");
        kernel_module::module_call(
            "dev",
            &DevRequest::RegisterDev(&(self as &'static dyn Device)),
        );
        Ok(())
    }
}

impl Device for PL011 {
    fn name(&self) -> &'static str {
        "tty.serial"
    }

    fn read(&self, _offset: usize, buf: &mut [u8]) -> usize {
        for i in 0..buf.len() {
            buf[i] = match self.uart().getchar(false) {
                Some(c) => c as u8,
                None => return i,
            };
        }
        0
    }
}

#[repr(C)]
pub struct UART0 {
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

impl UART0 {
    // fn transmit_fifo_full(&self) -> bool {
    //     self.fr.get() & (1 << 5) != 0
    // }

    fn receive_fifo_empty(&self) -> bool {
        self.fr.get() & (1 << 4) != 0
    }

    fn getchar(&mut self, block: bool) -> Option<char> {
        if self.receive_fifo_empty() {
            if !block {
                return None;
            }
            while self.receive_fifo_empty() {
                core::hint::spin_loop();
            }
        }
        let mut ret = self.dr.get() as u8 as char;
        if ret == '\r' {
            ret = '\n';
        }
        // if ret as u8 == 127 {
        //     ret = 0x8u8 as _;
        // }
        Some(ret)
    }

    // fn putchar(&mut self, c: char) {
    //     while self.transmit_fifo_full() {}
    //     self.dr.set(c as u8 as u32);
    // }

    fn init(&mut self) {
        self.cr.set(0);
        self.icr.set(0);
        self.ibrd.set(26);
        self.fbrd.set(3);
        self.lcrh.set(0b11 << 5);
        self.imsc.set(1 << 4);
        self.cr.set((1 << 0) | (1 << 8) | (1 << 9));
    }
}
