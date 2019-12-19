// mod device;
use crate::mm::*;

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
    fn alloc_frame<S: PageSize>() -> Frame<S>;
    fn dealloc_frame<S: PageSize>(frame: Frame<S>);
    fn map<S: PageSize>(page: Page<S>, frame: Frame<S>, flags: PageFlags);
    fn translate(address: Address<V>) -> Option<(Address<P>, PageFlags)>;
    fn update_flags<S: PageSize>(page: Page<S>, flags: PageFlags);
    fn unmap<S: PageSize>(page: Page<S>);
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

pub trait AbstractLogger: Sized {
    fn put(c: char);
}

pub trait AbstractArch: Sized {
    type Interrupt: AbstractInterruptController;
    type Timer: AbstractTimer;
    type MemoryManager: AbstractMemoryManager;
    type Context: AbstractContext;
    type Logger: AbstractLogger;

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
    pub type MemoryManager = <SelectedArch as AbstractArch>::MemoryManager;
    pub type Logger = <SelectedArch as AbstractArch>::Logger;
}


/// Entry point for the low-level boot code
#[no_mangle]
#[naked]
pub unsafe extern fn _start() -> ! {
    Target::Arch::_start()
}
