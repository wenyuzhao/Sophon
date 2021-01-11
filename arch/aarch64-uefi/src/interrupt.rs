use crate::drivers::gic::GIC;

use super::gic::*;
use cortex_a::barrier;
use super::exception::*;
use proton_kernel::arch::*;
use core::intrinsics::volatile_store;

pub struct InterruptController;

static mut INTERRUPT_HANDLERS: [Option<InterruptHandler>; 256] = [None; 256];

pub fn handle_interrupt(kind: InterruptId, exception_frame: &mut ExceptionFrame) -> isize {
    unreachable!()
}

impl AbstractInterruptController for InterruptController {
    fn init() {
        GIC.init();
    }

    fn is_enabled() -> bool {
        unsafe {
            let daif: usize;
            llvm_asm!("mrs $0, DAIF":"=r"(daif));
            daif & (1 << 7) == 0
        }
    }

    fn enable() {
        unsafe { llvm_asm!("msr daifclr, #2") };
    }

    fn disable() {
        unsafe { llvm_asm!("msr daifset, #2") };
    }

    fn set_handler(id: InterruptId, handler: Option<InterruptHandler>) {
        unsafe {
            INTERRUPT_HANDLERS[id as usize] = handler;
        }
    }
}
