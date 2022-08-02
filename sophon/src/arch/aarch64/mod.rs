mod context;
mod exception;

use super::{Arch, TargetArch};
use context::AArch64Context;

pub struct AArch64;

impl Arch for AArch64 {
    type Context = AArch64Context;

    fn init() {
        interrupt::disable();
    }

    fn setup_interrupt_table() {
        unsafe {
            exception::setup_vbar();
        }
    }
}

#[allow(unused)]
pub const fn create() -> TargetArch {
    AArch64
}
