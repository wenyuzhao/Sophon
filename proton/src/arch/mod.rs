use crate::{
    memory::page_table::PageTable,
    task::Message,
    utils::{address::*, page::Frame},
};
use alloc::boxed::Box;
use fdt::Fdt;

#[repr(usize)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum InterruptId {
    Timer = 0,
    Soft = 1,
    PageFault = 2,
}

pub type InterruptHandler = Box<dyn Fn(usize, usize, usize, usize, usize, usize) -> isize>;

static mut INTERRUPT_HANDLERS: [Option<InterruptHandler>; 3] = [None, None, None];

pub trait ArchInterrupt: 'static + Sized {
    fn is_enabled() -> bool;
    fn enable();
    fn disable();
}

pub trait ArchInterruptController {
    fn start_timer(&self);

    fn get_active_irq(&self) -> usize;

    fn notify_end_of_interrupt(&self);

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
    fn set_page_table(&mut self, page_table: &'static mut PageTable);
    fn set_response_message(&mut self, m: Message);
    fn set_response_status(&mut self, s: isize);

    unsafe extern "C" fn return_to_user(&mut self) -> !;
    unsafe fn enter_usermode(
        entry: extern "C" fn(_argc: isize, _argv: *const *const u8),
        sp: Address,
        page_table: &mut PageTable,
    ) -> !;
}

pub trait Arch {
    type Context: ArchContext;
    type Interrupt: ArchInterrupt;

    fn init(device_tree: &Fdt);
    fn interrupt() -> &'static dyn ArchInterruptController;
    #[inline]
    fn uninterruptable<R, F: FnOnce() -> R>(f: F) -> R {
        let enabled = Self::Interrupt::is_enabled();
        if enabled {
            Self::Interrupt::disable();
        }
        let ret = f();
        if enabled {
            Self::Interrupt::enable();
        }
        ret
    }

    fn get_current_page_table() -> Frame;
    fn set_current_page_table(page_table: Frame);
}

pub type TargetArch = impl Arch;

#[cfg(target_arch = "aarch64")]
mod aarch64;
