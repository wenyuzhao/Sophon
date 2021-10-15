mod context;
mod drivers;
mod exception;

use super::{Arch, ArchInterruptController, TargetArch};
use alloc::boxed::Box;
use context::AArch64Context;
use fdt::Fdt;

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
}

#[allow(unused)]
pub const fn create() -> TargetArch {
    AArch64
}
