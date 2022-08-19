use crate::arch::{Arch, TargetArch};
use core::ops::Deref;
use interrupt::IRQHandler;

static mut INTERRUPT_IMPL: &'static dyn interrupt::InterruptController =
    &UnimplementedInterruptController;

pub static INTERRUPT: InterruptController = InterruptController;

pub struct InterruptController;

impl InterruptController {
    pub fn set_interrupt_controller(
        &self,
        interrupt_controller: &'static dyn interrupt::InterruptController,
    ) {
        unsafe {
            INTERRUPT_IMPL = interrupt_controller;
        }
        TargetArch::setup_interrupt_table();
    }
}

impl Deref for InterruptController {
    type Target = dyn interrupt::InterruptController;
    fn deref(&self) -> &Self::Target {
        unsafe { INTERRUPT_IMPL }
    }
}

struct UnimplementedInterruptController;

impl interrupt::InterruptController for UnimplementedInterruptController {
    fn init(&self, _bsp: bool) {
        unimplemented!()
    }
    fn get_active_irq(&self) -> Option<usize> {
        unimplemented!()
    }
    fn enable_irq(&self, _irq: usize) {
        unimplemented!()
    }
    fn disable_irq(&self, _irq: usize) {
        unimplemented!()
    }
    fn interrupt_begin(&self) {
        unimplemented!()
    }
    fn interrupt_end(&self) {
        unimplemented!()
    }
    fn get_irq_handler(&self, _irq: usize) -> Option<&IRQHandler> {
        unimplemented!()
    }
    fn set_irq_handler(&self, _irq: usize, _handler: IRQHandler) {
        unimplemented!()
    }
}
