use crate::arch::{Arch, TargetArch};
use core::ops::Deref;

static mut INTERRUPT_IMPL: Option<&'static dyn interrupt::InterruptController> = None;

pub static INTERRUPT: InterruptController = InterruptController;

pub struct InterruptController;

impl InterruptController {
    pub fn set_interrupt_controller(
        &self,
        interrupt_controller: &'static dyn interrupt::InterruptController,
    ) {
        unsafe {
            INTERRUPT_IMPL = Some(interrupt_controller);
        }
        TargetArch::setup_interrupt_table();
    }
}

impl Deref for InterruptController {
    type Target = dyn interrupt::InterruptController;
    fn deref(&self) -> &Self::Target {
        unsafe { INTERRUPT_IMPL.unwrap_unchecked() }
    }
}
