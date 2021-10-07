mod context;
mod drivers;
mod exception;

use super::{Arch, ArchInterruptController, TargetArch};
use alloc::boxed::Box;
use context::AArch64Context;
use core::ops::Range;
use cortex_a::asm::barrier::*;
use cortex_a::registers::TTBR0_EL1;
use fdt::Fdt;
use memory::{
    address::{Address, MemoryKind},
    page::Frame,
};
use tock_registers::interfaces::Readable;

static mut INTERRUPT_CONTROLLER: Option<Box<dyn ArchInterruptController>> = None;

pub struct AArch64;

impl Arch for AArch64 {
    type Context = AArch64Context;

    fn init(device_tree: &Fdt) {
        interrupt::disable();
        unsafe {
            drivers::init(device_tree);
        }
    }

    fn interrupt() -> &'static dyn ArchInterruptController {
        unsafe { &**INTERRUPT_CONTROLLER.as_ref().unwrap() }
    }

    fn get_current_page_table() -> Frame {
        Frame::new((TTBR0_EL1.get() as usize).into())
    }

    fn set_current_page_table(page_table: Frame) {
        unsafe {
            asm! {
                "
                    msr	ttbr0_el1, {v}
                    tlbi vmalle1is
                    DSB ISH
                    isb
                ",
                v = in(reg) page_table.start().as_usize()
            }
        }
    }

    fn clear_cache<K: MemoryKind>(range: Range<Address<K>>) {
        const CACHE_LINE_SIZE: usize = 64;
        let start = range.start.align_down(CACHE_LINE_SIZE);
        let end = if range.end.is_aligned_to(CACHE_LINE_SIZE) {
            range.end
        } else {
            range.end.align_up(CACHE_LINE_SIZE)
        };
        unsafe {
            dsb(SY);
            isb(SY);
            for cache_line in (start..end).step_by(CACHE_LINE_SIZE) {
                asm!(
                    "
                        dc cvau, x0
                        ic ivau, x0
                    ",
                    in("x0") cache_line.as_usize(),
                );
            }
            dsb(SY);
            isb(SY);
        }
    }
}

#[allow(unused)]
pub const fn create() -> TargetArch {
    AArch64
}
