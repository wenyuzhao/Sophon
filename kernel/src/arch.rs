use core::ops::*;
use proton::memory::*;
use alloc::boxed::Box;
use crate::kernel_process::KernelTask;



#[repr(usize)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum InterruptId {
    Timer = 0,
    Soft = 1,
    PageFault = 2,
}

// pub type InterruptHandler = fn(a: usize, b: usize, c: usize, d: usize, e: usize, f: usize) -> isize;
pub type InterruptHandler = Box<dyn Fn(usize, usize, usize, usize, usize, usize) -> isize>;

pub trait AbstractInterruptController: Sized + 'static {
    fn init();
    
    fn is_enabled() -> bool;
    fn enable();
    fn disable();

    fn set_handler(id: InterruptId, handler: Option<InterruptHandler>);

    #[inline]
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

// pub struct TemporaryPage<S: PageSize>(Page<S>);

// impl <S: PageSize> Deref for TemporaryPage<S> {
//     type Target = Page<S>;
//     fn deref(&self) -> &Self::Target {
//         &self.0
//     }
// }

// impl <S: PageSize> Drop for TemporaryPage<S> {
//     fn drop(&mut self) {
//         Target::MemoryManager::unmap(self.0);
//     }
// }

pub trait AbstractMemoryManager: Sized {
    fn alloc_frame<S: PageSize>() -> Frame<S>;
    fn dealloc_frame<S: PageSize>(frame: Frame<S>);
    fn map<S: PageSize>(page: Page<S>, frame: Frame<S>, flags: PageFlags);
    fn translate(address: Address<V>) -> Option<(Address<P>, PageFlags)>;
    fn update_flags<S: PageSize>(page: Page<S>, flags: PageFlags);
    fn unmap<S: PageSize>(page: Page<S>);
    // fn map_temporarily<S: PageSize>(page: Page<S>, frame: Frame<S>, flags: PageFlags) -> TemporaryPage<S>;
}

pub trait AbstractTimer: Sized {
    fn init();
    fn wait(ms: usize);
}

pub trait AbstractContext: Sized + 'static {
    fn empty() -> Self;
    fn new(entry: *const extern fn(a: *mut ()) -> !, ctx: *mut ()) -> Self;
    // fn fork(&self) -> Self;
    fn set_response_message(&mut self, m: crate::task::Message);
    fn set_response_status(&mut self, s: isize);
    unsafe extern fn return_to_user(&mut self) -> !;
    unsafe fn enter_usermode(entry: extern fn(_argc: isize, _argv: *const *const u8), sp: Address) -> !;
}

pub trait AbstractLogger: Sized + 'static {
    fn put(c: char);
}

pub trait AbstractKernelHeap: Sized + 'static {
    // const RANGE: (Address, Address);
    fn init();
}

pub trait AbstractArch: Sized + 'static {
    type Interrupt: AbstractInterruptController;
    type Timer: AbstractTimer;
    type MemoryManager: AbstractMemoryManager;
    type Context: AbstractContext;
    type Logger: AbstractLogger;
    type Heap: AbstractKernelHeap;
    
    fn create_idle_task() -> Box<dyn KernelTask>;
}
