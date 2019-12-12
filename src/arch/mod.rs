// mod device;

#[repr(usize)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum InterruptId {
    Timer = 0,
    Soft = 1,
    PageFault = 2,
}

pub type InterruptHandler = fn(a: usize, b: usize, c: usize, d: usize, e: usize, f: usize) -> isize;

pub trait AbstractInterruptController: Sized {
    fn initialize();
    
    fn is_enabled() -> bool;
    fn enable();
    fn disable();

    fn set_handler(id: InterruptId, handler: Option<InterruptHandler>);

    fn uninterruptable<R, F: FnOnce() -> R>(f: F) -> R {
        let enabled = Self::is_enabled();
        if enabled {
            Self::disable();
        }
        let ret = f();
        if enabled {
            Self::enable();
        }
        ret
    }
}

pub trait AbstractMemoryManager: Sized {
    
}

pub trait AbstractTimer: Sized {
    
}

pub trait AbstractArch: Sized {
    // type MemoryManager: AbstractMemoryManager;
    type Interrupt: AbstractInterruptController;
    // type Timer: AbstractTimer;

    /// Platform initialization code
    /// Initialize: VirtualMemory/ExceptionVectorTable/...
    #[inline(always)]
    #[naked]
    unsafe fn _start() -> !;
}

#[cfg(target_arch="aarch64")]
mod aarch64;
#[cfg(target_arch="aarch64")]
use aarch64::AArch64 as TargetArch;

static mut BOOTED: bool = false;
fn set_booted() {
    unsafe { BOOTED = true }
}
pub fn booted() -> bool {
    unsafe { BOOTED }
}

pub mod Target {
    use super::*;
    pub type Arch = TargetArch;
    pub type Interrupt = <TargetArch as AbstractArch>::Interrupt;
}

/// Entry point for the low-level boot code
#[no_mangle]
#[naked]
pub unsafe extern fn _start() -> ! {
    TargetArch::_start()
}
