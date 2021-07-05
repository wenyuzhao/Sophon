use crate::utils::address::*;
use alloc::boxed::Box;
use device_tree::DeviceTree;

#[repr(usize)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum InterruptId {
    Timer = 0,
    Soft = 1,
    PageFault = 2,
}

pub type InterruptHandler = Box<dyn Fn(usize, usize, usize, usize, usize, usize) -> isize>;

static mut INTERRUPT_HANDLERS: [Option<InterruptHandler>; 3] = [None, None, None];

pub trait ArchInterrupt {
    fn is_enabled(&self) -> bool;
    fn enable(&self);
    fn disable(&self);
    fn start_timer(&self);
    fn handle(&self, id: InterruptId, args: &[usize]) -> usize {
        let mut x = [0usize; 6];
        for i in 0..args.len() {
            x[i] = args[i];
        }
        if let Some(handler) = unsafe { &INTERRUPT_HANDLERS[id as usize] } {
            handler(x[0], x[1], x[2], x[3], x[4], x[5]);
            0
            // result
            // exception_frame.x0 = unsafe { ::core::mem::transmute(result) };
        } else {
            log!("Interrupt<{:?}> has no handler!", id);
            0
        }
    }
    fn set_handler(&self, id: InterruptId, handler: Option<InterruptHandler>) {
        unsafe {
            INTERRUPT_HANDLERS[id as usize] = handler;
        }
    }
}

pub trait ArchContext: Sized + 'static {
    fn empty() -> Self;
    fn new(entry: *const extern "C" fn(a: *mut ()) -> !, ctx: *mut ()) -> Self;
    fn set_response_message(&mut self, m: crate::task::Message);
    fn set_response_status(&mut self, s: isize);
    unsafe extern "C" fn return_to_user(&mut self) -> !;
    unsafe fn enter_usermode(
        entry: extern "C" fn(_argc: isize, _argv: *const *const u8),
        sp: Address,
    ) -> !;
}

pub trait Arch {
    type Context: ArchContext;
    fn init(device_tree: &DeviceTree);
    fn interrupt() -> &'static dyn ArchInterrupt;
    fn uninterruptable<R, F: FnOnce() -> R>(f: F) -> R;
}

pub type TargetArch = impl Arch;

#[cfg(target_arch = "aarch64")]
mod aarch64;
