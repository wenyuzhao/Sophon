mod context;
mod drivers;
mod exception;

use super::{Arch, ArchInterruptController, TargetArch};
use alloc::boxed::Box;
use context::AArch64Context;
use cortex_a::registers::TTBR0_EL1;
use fdt::Fdt;
use memory::page::Frame;
use tock_registers::interfaces::Readable;

static mut INTERRUPT_CONTROLLER: Option<Box<dyn ArchInterruptController>> = None;

pub struct AArch64;

impl Arch for AArch64 {
    type Context = AArch64Context;

    fn init(device_tree: &Fdt) {
        interrupt::disable();
        unsafe {
            drivers::init(device_tree);
        }
    }

    fn interrupt() -> &'static dyn ArchInterruptController {
        unsafe { &**INTERRUPT_CONTROLLER.as_ref().unwrap() }
    }

    fn get_current_page_table() -> Frame {
        Frame::new((TTBR0_EL1.get() as usize).into())
    }

    fn set_current_page_table(page_table: Frame) {
        unsafe {
            asm! {
                "
                    msr	ttbr0_el1, {v}
                    tlbi vmalle1is
                    DSB ISH
                    isb
                ",
                v = in(reg) page_table.start().as_usize()
            }
        }
    }
}

#[allow(unused)]
pub const fn create() -> TargetArch {
    AArch64
}
