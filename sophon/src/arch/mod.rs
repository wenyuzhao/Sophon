use boot::BootInfo;
use interrupt::InterruptController;
use memory::address::*;
use memory::page_table::PageTable;

#[allow(unused)]
#[inline]
pub(self) fn handle_irq(irq: usize) -> isize {
    if let Some(handler) = TargetArch::interrupt().get_irq_handler(irq) {
        handler()
    } else {
        log!("IRQ #{:?} has no handler!", irq);
        0
    }
}

pub trait ArchContext: Sized + 'static {
    fn empty() -> Self;
    fn new(entry: *const extern "C" fn(ctx: *mut ()) -> !, ctx: *mut ()) -> Self;
    fn set_response_status(&self, s: isize);

    unsafe extern "C" fn return_to_user(&self) -> !;
    unsafe fn enter_usermode(
        entry: extern "C" fn(_argc: isize, _argv: *const *const u8),
        sp: Address,
        page_table: &mut PageTable,
        argc: isize,
        argv: *const *const u8,
    ) -> !;
}

static mut INTERRUPT_CONTROLLER: Option<&'static dyn InterruptController> = None;

pub trait Arch {
    type Context: ArchContext;

    fn init(boot_info: &'static BootInfo);

    fn interrupt() -> &'static dyn InterruptController {
        unsafe { &**INTERRUPT_CONTROLLER.as_ref().unwrap() }
    }

    fn set_interrupt_controller(controller: &'static dyn InterruptController) {
        unsafe { INTERRUPT_CONTROLLER = Some(controller) }
        Self::setup_interrupt_table();
    }

    fn setup_interrupt_table();

    fn halt(code: i32) -> !;

    fn current_cpu() -> usize;

    fn num_cpus() -> usize;
}

pub type TargetArch = impl Arch;

#[cfg(target_arch = "aarch64")]
mod aarch64;

#[cfg(target_arch = "x86_64")]
mod x64;
