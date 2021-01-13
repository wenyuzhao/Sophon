use device_tree::DeviceTree;
use proton::memory::Address;


pub trait ArchInterrupt {
    fn is_enabled(&self) -> bool;
    fn enable(&self);
    fn disable(&self);
}

pub trait ArchContext: Sized + 'static {
    fn empty() -> Self;
    fn new(entry: *const extern fn(a: *mut ()) -> !, ctx: *mut ()) -> Self;
    fn set_response_message(&mut self, m: crate::task::Message);
    fn set_response_status(&mut self, s: isize);
    unsafe extern fn return_to_user(&mut self) -> !;
    unsafe fn enter_usermode(entry: extern fn(_argc: isize, _argv: *const *const u8), sp: Address) -> !;
}

pub trait Arch {
    type Context: ArchContext;
    fn init(device_tree: &DeviceTree);
    fn interrupt() -> &'static dyn ArchInterrupt;
    fn uninterruptable<R, F: FnOnce() -> R>(f: F) -> R;
}

pub type TargetArch = impl Arch;

#[cfg(target_arch="aarch64")]
mod aarch64;
