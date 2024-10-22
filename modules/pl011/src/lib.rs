#![feature(format_args_nl)]
#![no_std]

#[allow(unused)]
#[macro_use]
extern crate log;
extern crate alloc;

use alloc::boxed::Box;
use core::fmt;
use crossbeam::queue::SegQueue;
use dev::{DevRequest, Device};
use kernel_module::monitor::SysMonitor;
use kernel_module::{kernel_module, KernelModule, SERVICE};
use memory::{page::Frame, volatile::Volatile};
use spin::{Lazy, RwLock};

#[kernel_module]
pub static PL011: PL011 = PL011 {
    uart: RwLock::new(core::ptr::null_mut()),
    buffer: SegQueue::new(),
    monitor: Lazy::new(|| SERVICE.create_monitor()),
};

unsafe impl Send for PL011 {}
unsafe impl Sync for PL011 {}

pub struct PL011 {
    pub uart: RwLock<*mut UART0>,
    pub buffer: SegQueue<u8>,
    monitor: Lazy<Box<dyn SysMonitor>>,
}

impl PL011 {
    fn uart(&self) -> &mut UART0 {
        unsafe { &mut **self.uart.read() }
    }
}

impl KernelModule for PL011 {
    fn init(&'static mut self) -> anyhow::Result<()> {
        let devtree = SERVICE.get_device_tree().unwrap();
        let node = devtree.compatible("arm,pl011").unwrap();
        let uart_frame = node.translate(node.regs().unwrap().next().unwrap().start);
        let uart_page = SERVICE.map_device_page(Frame::new(uart_frame));
        let uart = unsafe { &mut *(uart_page.start().as_mut_ptr() as *mut UART0) };
        uart.init();
        *self.uart.write() = uart;
        SERVICE.set_sys_logger(&raw mut UART_LOGGER);
        // Initialize interrupts
        let irq = node.interrupts().unwrap().next().unwrap().0;
        SERVICE.interrupt_controller().set_irq_handler(
            irq,
            Box::new(|| {
                PL011.monitor.lock();
                while !self.uart().receive_fifo_empty() {
                    let c = self.uart().dr.get() as u8;
                    self.buffer.push(c);
                }
                PL011.monitor.notify_all();
                PL011.monitor.unlock();
                0
            }),
        );
        SERVICE.interrupt_controller().enable_irq(irq);
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

    fn read(&self, _offset: usize, buf: &mut [u8]) -> Option<usize> {
        for i in 0..buf.len() {
            buf[i] = self.uart().getchar(true).unwrap() as _;
        }
        Some(buf.len())
    }

    fn write(&self, _offset: usize, buf: &[u8]) -> Option<usize> {
        for i in 0..buf.len() {
            self.uart().putchar(buf[i] as _);
        }
        Some(buf.len())
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
    fn transmit_fifo_full(&self) -> bool {
        self.fr.get() & (1 << 5) != 0
    }

    fn receive_fifo_empty(&self) -> bool {
        self.fr.get() & (1 << 4) != 0
    }

    fn getchar(&mut self, block: bool) -> Option<char> {
        if PL011.buffer.is_empty() {
            if !block {
                return None;
            } else {
                PL011.monitor.lock();
                while PL011.buffer.is_empty() {
                    PL011.monitor.wait();
                }
                PL011.monitor.unlock();
            }
        }
        let mut c = PL011.buffer.pop().unwrap() as char;
        if c == '\r' {
            c = '\n';
        }
        Some(c)
        // if ret as u8 == 127 {
        //     ret = 0x8u8 as _;
        // }
    }

    fn putchar(&mut self, c: char) {
        while self.transmit_fifo_full() {
            core::hint::spin_loop();
        }
        if c == '\n' {
            self.dr.set('\r' as u8 as u32);
        }
        while self.transmit_fifo_full() {}
        self.dr.set(c as u8 as u32);
    }

    fn init(&mut self) {
        self.cr.set(0);
        self.icr.set(0);
        self.ibrd.set(26);
        self.fbrd.set(3);
        self.lcrh.set(0b11 << 5);
        self.cr.set((1 << 0) | (1 << 8) | (1 << 9));
        self.imsc.set(1 << 4);
    }
}

static mut UART_LOGGER: UARTLogger = UARTLogger(UARTLoggerInner);

pub struct UARTLoggerInner;

impl core::fmt::Write for UARTLoggerInner {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.chars() {
            if c == '\n' {
                PL011.uart().putchar('\r');
            }
            PL011.uart().putchar(c);
        }
        Ok(())
    }
}

pub struct UARTLogger(UARTLoggerInner);

impl core::fmt::Write for UARTLogger {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let _guard = interrupt::uninterruptible();
        self.0.write_str(s)
    }

    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> fmt::Result {
        let _guard = interrupt::uninterruptible();
        self.0.write_fmt(args)
    }
}
