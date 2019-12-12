mod start;
mod gic;
mod interrupt;
mod exception;
mod timer;

use super::*;
use crate::mm::paging;
use cortex_a::{asm, regs::*, barrier};

pub struct AArch64;

impl AbstractArch for AArch64 {
    type Interrupt = interrupt::InterruptController;
    type Timer = timer::Timer;

    #[inline(always)]
    #[naked]
    unsafe fn _start() -> ! {
        start::_start()
    }
}
