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
    fn init();
    
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
    fn init();
}

pub trait AbstractTimer: Sized {
    fn init();
}

pub trait AbstractContext: Sized {
    fn empty() -> Self;
    fn new(entry: *const extern fn() -> !) -> Self;
    fn fork(&self) -> Self;
    unsafe extern fn switch_to(&mut self, ctx: &Self);
}

pub trait AbstractArch: Sized {
    type Interrupt: AbstractInterruptController;
    type Timer: AbstractTimer;
    type Context: AbstractContext;

    /// Platform initialization code
    /// Initialize: VirtualMemory/ExceptionVectorTable/...
    #[inline(always)]
    #[naked]
    unsafe fn _start() -> !;
}

#[cfg(target_arch="aarch64")]
mod aarch64;
#[cfg(target_arch="aarch64")]
pub use aarch64::AArch64 as SelectedArch;

static mut BOOTED: bool = false;
fn set_booted() {
    unsafe { BOOTED = true }
}
pub fn booted() -> bool {
    unsafe { BOOTED }
}

pub mod Target {
    use super::*;
    pub type Arch = SelectedArch;
    pub type Interrupt = <SelectedArch as AbstractArch>::Interrupt;
    pub type Timer = <SelectedArch as AbstractArch>::Timer;
    pub type Context = <SelectedArch as AbstractArch>::Context;
}


/// Entry point for the low-level boot code
#[no_mangle]
#[naked]
pub unsafe extern fn _start() -> ! {
    Target::Arch::_start()
}
