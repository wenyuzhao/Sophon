mod start;
mod gic;
mod interrupt;
mod exception;
mod timer;
mod context;
mod mm;

use super::*;

pub struct AArch64;

impl AbstractArch for AArch64 {
    type Interrupt = interrupt::InterruptController;
    type Timer = timer::Timer;
    type Context = context::Context;
    type MemoryManager = mm::MemoryManager;

    #[inline(always)]
    #[naked]
    unsafe fn _start() -> ! {
        start::_start()
    }
}
