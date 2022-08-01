use alloc::boxed::Box;
use devtree::DeviceTree;
use memory::address::*;
use memory::page_table::PageTable;

pub type IRQHandler = Box<dyn Fn() -> isize>;
pub type InterruptHandler = Box<dyn Fn(usize, usize, usize, usize, usize, usize) -> isize>;

const MAX_IRQS: usize = 256;
const IRQ_UNINIT: Option<IRQHandler> = None;
static mut IRQ_HANDLERS: [Option<IRQHandler>; MAX_IRQS] = [IRQ_UNINIT; MAX_IRQS];
static mut SYSCALL_HANDLER: Option<InterruptHandler> = None;

#[allow(unused)]
#[inline]
pub(self) fn handle_irq(irq: usize) -> isize {
    if let Some(handler) = unsafe { IRQ_HANDLERS[irq].as_ref() } {
        handler()
    } else {
        log!("IRQ #{:?} has no handler!", irq);
        0
    }
}

pub trait ArchInterruptController {
    fn get_active_irq(&self) -> usize;

    fn set_irq_handler(&self, irq: usize, handler: IRQHandler) {
        unsafe {
            IRQ_HANDLERS[irq] = Some(handler);
        }
    }

    fn enable_irq(&self, irq: usize);

    fn disable_irq(&self, irq: usize);

    fn notify_end_of_interrupt(&self);

    fn set_syscall_handler(&self, handler: Option<InterruptHandler>) {
        unsafe {
            SYSCALL_HANDLER = handler;
        }
    }
}

pub trait ArchContext: Sized + 'static {
    fn empty() -> Self;
    fn new(entry: *const extern "C" fn(a: *mut ()) -> !, ctx: *mut ()) -> Self;
    fn set_response_status(&self, s: isize);

    unsafe extern "C" fn return_to_user(&self) -> !;
    unsafe fn enter_usermode(
        entry: extern "C" fn(_argc: isize, _argv: *const *const u8),
        sp: Address,
        page_table: &mut PageTable,
    ) -> !;
}

pub trait Arch {
    type Context: ArchContext;

    fn init(device_tree: &'static DeviceTree<'static, 'static>);
    fn interrupt() -> &'static dyn ArchInterruptController;
}

pub type TargetArch = impl Arch;

#[cfg(target_arch = "aarch64")]
mod aarch64;

#[cfg(target_arch = "x86_64")]
mod x64;
