mod context;
mod drivers;
mod exception;

use super::{Arch, ArchInterruptController, TargetArch};
use alloc::boxed::Box;
use context::AArch64Context;
use fdt::Fdt;

static mut INTERRUPT_CONTROLLER: Option<Box<dyn ArchInterruptController>> = None;

static DEVICE_TREE: spin::Mutex<Option<Fdt<'static>>> = spin::Mutex::new(None);

pub struct AArch64;

impl Arch for AArch64 {
    type Context = AArch64Context;

    fn init(device_tree: Fdt<'static>) {
        interrupt::disable();
        unsafe {
            drivers::init(&device_tree);
        }
        *DEVICE_TREE.lock() = Some(device_tree);
    }

    fn interrupt() -> &'static dyn ArchInterruptController {
        unsafe { &**INTERRUPT_CONTROLLER.as_ref().unwrap() }
    }

    fn device_tree() -> Option<fdt::Fdt<'static>> {
        *DEVICE_TREE.lock()
    }
}

#[allow(unused)]
pub const fn create() -> TargetArch {
    AArch64
}
