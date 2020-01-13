pub mod start;
pub mod gic;
pub mod interrupt;
pub mod exception;
pub mod timer;
pub mod context;
pub mod mm;
pub mod uart;
pub mod constants;

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
