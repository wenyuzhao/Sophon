mod context;
mod exception;

use super::{Arch, TargetArch};
use crate::memory::kernel::KERNEL_MEMORY_MAPPER;
use boot::BootInfo;
use context::AArch64Context;
use core::arch::asm;

static mut SHUTDOWN: Option<extern "C" fn() -> !> = None;

pub struct AArch64;

impl Arch for AArch64 {
    type Context = AArch64Context;

    fn init(boot_info: &'static BootInfo) {
        unsafe { SHUTDOWN = boot_info.shutdown };
        interrupt::disable();
    }

    fn setup_interrupt_table() {
        unsafe {
            exception::setup_vbar();
        }
    }

    fn halt(code: i32) -> ! {
        // Try QEMU exit service
        if cfg!(feature = "qemu") {
            unsafe {
                let payload = [0x20026u64, code as u64];
                let paddr = KERNEL_MEMORY_MAPPER
                    .translate(payload.as_ptr().into())
                    .unwrap();
                asm!(
                    "hlt #0xF000",
                    in("x0") 0x18,
                    in("x1") paddr.as_usize(),
                    options(nostack)
                );
            }
        }
        // TODO: Try PSCI system off
        // Try shutdown service from the bootloader
        unsafe {
            if let Some(shutdown) = SHUTDOWN {
                let _guard = KERNEL_MEMORY_MAPPER.with_kernel_address_space();
                shutdown()
            }
        }
        // Spin
        error!("ERROR: Failed to shutdown.");
        loop {
            unsafe { asm!("wfe") };
        }
    }
}

#[allow(unused)]
pub const fn create() -> TargetArch {
    AArch64
}
