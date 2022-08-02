#![feature(format_args_nl)]
#![feature(default_alloc_error_handler)]
#![feature(box_syntax)]
#![no_std]

#[allow(unused)]
#[macro_use]
extern crate log;
extern crate alloc;

use core::arch::asm;
use cortex_a::registers::*;
use kernel_module::{kernel_module, KernelModule, SERVICE};
use tock_registers::interfaces::{Readable, Writeable};

const TIMER_INTERRUPT_FREQUENCY: usize = 60; // Hz

#[kernel_module]
pub static GIC_TIMER: GICTimer = GICTimer;

unsafe impl Send for GICTimer {}
unsafe impl Sync for GICTimer {}

pub struct GICTimer;

impl GICTimer {
    fn get_timer_irq(&self) -> usize {
        let devtree = SERVICE.get_device_tree().unwrap();
        let node = devtree.compatible("arm,armv7-timer").unwrap();
        let (irq, _) = node.interrupts().unwrap().skip(1).next().unwrap();
        irq
    }

    fn set_timer_handler(&self, irq: usize) {
        SERVICE.set_irq_handler(irq, box || {
            // Update compare value
            let step = CNTFRQ_EL0.get() as u64 / TIMER_INTERRUPT_FREQUENCY as u64;
            CNTP_TVAL_EL0.set(step as u64);
            SERVICE.schedule();
        });
    }

    fn start_timer(&self, irq: usize) {
        unsafe {
            asm!("dsb SY");
            SERVICE.enable_irq(irq);
            let n_cntfrq: usize = CNTFRQ_EL0.get() as _;
            // assert!(n_cntfrq % TIMER_INTERRUPT_FREQUENCY == 0);
            let clock_ticks_per_timer_irq = n_cntfrq / TIMER_INTERRUPT_FREQUENCY;
            CNTP_TVAL_EL0.set(clock_ticks_per_timer_irq as u64);
            CNTP_CTL_EL0.set(1);
            asm!("dmb SY");
        }
    }
}

impl KernelModule for GICTimer {
    fn init(&mut self) -> anyhow::Result<()> {
        let irq = self.get_timer_irq();
        self.set_timer_handler(irq);
        self.start_timer(irq);
        Ok(())
    }
}
