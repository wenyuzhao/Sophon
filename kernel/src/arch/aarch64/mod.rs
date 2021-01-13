mod drivers;
mod exception;
mod context;

use alloc::boxed::Box;
use context::AArch64Context;
use device_tree::DeviceTree;
use crate::boot_driver::BootDriver;
use super::{TargetArch, Arch, ArchInterrupt};

static mut INTERRUPT_CONTROLLER: Option<Box<dyn ArchInterrupt>> = None;

pub struct AArch64;

impl Arch for AArch64 {
    type Context = AArch64Context;

    fn init(device_tree: &DeviceTree) {
        unsafe { llvm_asm!("msr daifset, #2") };

        {
            let uart = drivers::uart::UART.lock();
            uart.init_with_device_tree(device_tree);
            uart.putchar('@');
            uart.putchar('\n');
        }

        log!("uart initizlied");

        drivers::gic::GIC.init_with_device_tree(device_tree);
    }

    fn interrupt() -> &'static dyn ArchInterrupt {
        unsafe { &**INTERRUPT_CONTROLLER.as_ref().unwrap() }
    }


    #[inline]
    fn uninterruptable<R, F: FnOnce() -> R>(f: F) -> R {
        let enabled = unsafe {
            let daif: usize;
            llvm_asm!("mrs $0, DAIF":"=r"(daif));
            daif & (1 << 7) == 0
        };
        if enabled {
            unsafe { llvm_asm!("msr daifset, #2") };
        }
        let ret = f();
        if enabled {
            unsafe { llvm_asm!("msr daifclr, #2") };
        }
        ret
    }
}

pub const fn create() -> TargetArch {
    AArch64
}