use boot::BootInfo;
use memory::address::*;
use memory::page_table::PageTable;
use proc::Task;

#[allow(unused)]
#[inline]
pub(self) fn handle_irq(irq: usize) -> isize {
    if let Some(handler) = crate::modules::INTERRUPT.get_irq_handler(irq) {
        handler()
    } else {
        error!("IRQ #{:?} has no handler!", irq);
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

    fn of(task: &Task) -> &Self {
        unsafe { task.context.downcast_ref_unchecked() }
    }
}

pub trait Arch {
    type Context: ArchContext;

    fn init(boot_info: &'static BootInfo);

    fn setup_interrupt_table();

    fn halt(code: i32) -> !;
}

pub type TargetArch = impl Arch;

#[cfg(target_arch = "aarch64")]
mod aarch64;

#[cfg(target_arch = "x86_64")]
mod x64;
