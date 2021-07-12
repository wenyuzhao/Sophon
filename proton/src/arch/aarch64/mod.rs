mod context;
mod drivers;
mod exception;

use super::{Arch, ArchInterrupt, ArchInterruptController, TargetArch};
use crate::utils::page::Frame;
use alloc::boxed::Box;
use context::AArch64Context;
use cortex_a::regs::*;
use device_tree::DeviceTree;

static mut INTERRUPT_CONTROLLER: Option<Box<dyn ArchInterruptController>> = None;

pub struct AArch64Interrupt;

impl ArchInterrupt for AArch64Interrupt {
    fn is_enabled() -> bool {
        unsafe {
            let daif: usize;
            asm!("mrs {}, DAIF", out(reg) daif);
            daif & (1 << 7) == 0
        }
    }

    fn enable() {
        unsafe { asm!("msr daifclr, #2") };
    }

    fn disable() {
        unsafe { asm!("msr daifset, #2") };
    }
}

pub struct AArch64;

impl Arch for AArch64 {
    type Context = AArch64Context;
    type Interrupt = AArch64Interrupt;

    fn init(device_tree: &DeviceTree) {
        Self::Interrupt::disable();
        drivers::init(device_tree);
    }

    fn interrupt() -> &'static dyn ArchInterruptController {
        unsafe { &**INTERRUPT_CONTROLLER.as_ref().unwrap() }
    }

    #[inline]
    fn uninterruptable<R, F: FnOnce() -> R>(f: F) -> R {
        let enabled = unsafe {
            let daif: usize;
            asm!("mrs {}, DAIF", out(reg) daif);
            daif & (1 << 7) == 0
        };
        if enabled {
            unsafe { asm!("msr daifset, #2") };
        }
        let ret = f();
        if enabled {
            unsafe { asm!("msr daifclr, #2") };
        }
        ret
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
