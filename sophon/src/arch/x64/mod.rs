use super::{Arch, ArchContext, TargetArch};
use boot::BootInfo;
use memory::{address::Address, page_table::PageTable};

#[repr(C)]
pub struct X64Context;

impl ArchContext for X64Context {
    fn empty() -> Self {
        unimplemented!()
    }

    fn new(_entry: *const extern "C" fn(a: *mut ()) -> !, _ctx_ptr: *mut ()) -> Self {
        unimplemented!()
    }

    fn set_response_status(&self, _s: isize) {
        unimplemented!()
    }

    unsafe extern "C" fn return_to_user(&self) -> ! {
        unimplemented!()
    }

    unsafe fn enter_usermode(
        &self,
        _entry: extern "C" fn(_argc: isize, _argv: *const *const u8),
        _sp: Address,
        _page_table: &mut PageTable,
        _argc: isize,
        _argv: *const *const u8,
    ) -> ! {
        unimplemented!()
    }
}
pub struct X64;

impl Arch for X64 {
    type Context = X64Context;

    fn init(_boot_info: &'static BootInfo) {
        unimplemented!()
    }

    fn setup_interrupt_table() {
        unimplemented!()
    }

    fn halt(_code: i32) -> ! {
        unimplemented!()
    }
}

#[allow(unused)]
pub const fn create() -> TargetArch {
    X64
}
