mod start;
mod gic;
mod interrupt;
mod exception;
mod timer;
mod context;
mod mm;
mod uart;
mod constants;

use super::*;

pub struct AArch64;

impl AbstractArch for AArch64 {
    type Interrupt = interrupt::InterruptController;
    type Timer = timer::Timer;
    type Context = context::Context;
    type MemoryManager = mm::MemoryManager;
    type Logger = uart::UART0;

    #[inline(always)]
    #[naked]
    unsafe fn _start() -> ! {
        start::_start()
    }
}
