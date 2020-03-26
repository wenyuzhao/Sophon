
use super::*;
use proton_kernel::arch::AbstractArch;
// use interrupt::InterruptController';Task::<AArch64Kernel>

pub struct AArch64;

impl AbstractArch for AArch64 {
    type Interrupt = crate::interrupt::InterruptController;
    type Timer = crate::timer::Timer;
    type Context = crate::context::Context;
    type MemoryManager = crate::mm::MemoryManager;
    type Logger = crate::uart::UART0;
    type Heap = crate::heap::KernelHeap;
}
