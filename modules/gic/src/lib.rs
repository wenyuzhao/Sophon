#![feature(format_args_nl)]
#![feature(default_alloc_error_handler)]
#![feature(box_syntax)]
#![no_std]

#[allow(unused)]
#[macro_use]
extern crate log;
extern crate alloc;

use core::{
    arch::asm,
    cell::UnsafeCell,
    sync::atomic::{AtomicU32, Ordering},
};
use cortex_a::asm::barrier;
use interrupt::{IRQHandler, InterruptController};
use kernel_module::{kernel_module, KernelModule, SERVICE};
use memory::{page::Frame, volatile::*};

pub const IRQ_LINES: usize = 256;

#[repr(C)]
#[allow(non_snake_case)]
pub struct GICD {
    pub CTLR: Volatile<u32>,
    _0: PaddingForRange<0x0004, 0x0080>,
    pub IGROUPR: VolatileArrayForRange<u32, 0x0080, 0x00FC>,
    _1: PaddingForRange<0x00FC, 0x0100>,
    pub ISENABLER: VolatileArrayForRange<u32, 0x0100, 0x017C>,
    _2: PaddingForRange<0x017C, 0x0180>,
    pub ICENABLER: VolatileArrayForRange<u32, 0x0180, 0x01FC>,
    _3: PaddingForRange<0x01FC, 0x0200>,
    pub ISPENDR: VolatileArrayForRange<u32, 0x0200, 0x027C>,
    _4: PaddingForRange<0x027C, 0x0280>,
    pub ICPENDR: VolatileArrayForRange<u32, 0x0280, 0x02FC>,
    _5: PaddingForRange<0x02FC, 0x0300>,
    pub ISACTIVER: VolatileArrayForRange<u32, 0x0300, 0x037C>,
    _6: PaddingForRange<0x037C, 0x0380>,
    pub ICACTIVER: VolatileArrayForRange<u32, 0x0380, 0x03FC>,
    _7: PaddingForRange<0x03FC, 0x0400>,
    pub IPRIORITYR: VolatileArrayForRange<u32, 0x0400, 0x07E0>,
    _8: PaddingForRange<0x07E0, 0x0800>,
    pub ITARGETSR: VolatileArrayForRange<u32, 0x0800, 0x0BE0>,
    _9: PaddingForRange<0x0BE0, 0x0C00>,
    pub ICFGR: VolatileArrayForRange<u32, 0x0C00, 0x0CF8>,
    _10: PaddingForRange<0x0CF8, 0x0F00>,
    pub SGIR: Volatile<u32>, /* 0x0F00 */
}

#[allow(unused)]
impl GICD {
    pub const CTLR_DISABLE: u32 = 0 << 0;
    pub const CTLR_ENABLE: u32 = 1 << 0;
    pub const CTLR_ENABLE_GROUP0: u32 = 1 << 0;
    pub const CTLR_ENABLE_GROUP1: u32 = 1 << 1;
    pub const IPRIORITYRAULT: u32 = 0xA0;
    pub const IPRIORITYR_FIQ: u32 = 0x40;
    pub const ITARGETSR_CORE0: u32 = 1 << 0;
    pub const ICFGR_LEVEL_SENSITIVE: u32 = 0 << 1;
    pub const ICFGR_EDGE_TRIGGERED: u32 = 1 << 1;
    pub const SGIR_SGIINTID__MASK: u32 = 0x0F;
    pub const SGIR_CPU_TARGET_LIST__SHIFT: u32 = 16;
    pub const SGIR_TARGET_LIST_FILTER__SHIFT: u32 = 24;
}

#[repr(C)]
#[allow(non_snake_case)]
pub struct GICC {
    pub CTLR: Volatile<u32>, // 0x000
    pub PMR: Volatile<u32>,  // 0x004;
    _0: PaddingForRange<0x0008, 0x00C>,
    pub IAR: Volatile<u32>,  // 0x00C
    pub EOIR: Volatile<u32>, // 0x010
}

#[allow(unused)]
impl GICC {
    pub const CTLR_DISABLE: u32 = 0 << 0;
    pub const CTLR_ENABLE: u32 = 1 << 0;
    pub const CTLR_ENABLE_GROUP0: u32 = 1 << 0;
    pub const CTLR_ENABLE_GROUP1: u32 = 1 << 1;
    pub const CTLR_FIQ_ENABLE: u32 = 1 << 3;
    pub const PMR_PRIORITY: u32 = 0xF0 << 0;
    pub const IAR_INTERRUPT_ID__MASK: u32 = 0x3FF;
    pub const IAR_CPUID__SHIFT: u32 = 10;
    pub const IAR_CPUID__MASK: u32 = 3 << 10;
    pub const EOIR_EOIINTID__MASK: u32 = 0x3FF;
    pub const EOIR_CPUID__SHIFT: u32 = 10;
    pub const EOIR_CPUID__MASK: u32 = 3 << 10;
}

#[allow(non_snake_case)]
pub struct GIC {
    GICD: UnsafeCell<*mut GICD>,
    GICC: UnsafeCell<*mut GICC>,
    iar: AtomicU32,
}

unsafe impl Send for GIC {}
unsafe impl Sync for GIC {}

impl GIC {
    const fn new() -> Self {
        Self {
            GICD: UnsafeCell::new(core::ptr::null_mut()),
            GICC: UnsafeCell::new(core::ptr::null_mut()),
            iar: AtomicU32::new(0),
        }
    }

    fn gicd(&self) -> &'static mut GICD {
        unsafe { &mut **self.GICD.get() }
    }

    fn gicc(&self) -> &'static mut GICC {
        unsafe { &mut **self.GICC.get() }
    }

    fn init_gic(&self) {
        #[allow(non_snake_case)]
        let (GICD, GICC) = (self.gicd(), self.gicc());
        unsafe { barrier::dsb(barrier::SY) };
        unsafe {
            // Disable all interrupts
            GICD.CTLR.set(GICD::CTLR_DISABLE);
            for n in 0..(IRQ_LINES / 32) {
                GICD.ICENABLER[n].set(!0);
                GICD.ICPENDR[n].set(!0);
                GICD.ICACTIVER[n].set(!0);
            }
            // Connect interrupts to core#0
            for n in 0..(IRQ_LINES / 4) {
                GICD.IPRIORITYR[n].set(
                    GICD::IPRIORITYRAULT
                        | GICD::IPRIORITYRAULT << 8
                        | GICD::IPRIORITYRAULT << 16
                        | GICD::IPRIORITYRAULT << 24,
                );
                GICD.ITARGETSR[n].set(
                    GICD::ITARGETSR_CORE0
                        | GICD::ITARGETSR_CORE0 << 8
                        | GICD::ITARGETSR_CORE0 << 16
                        | GICD::ITARGETSR_CORE0 << 24,
                );
            }
            // set all interrupts to level triggered
            for n in 0..(IRQ_LINES / 16) {
                GICD.ICFGR[n].set(0);
            }
            // Enable GIC
            GICD.CTLR.set(GICD::CTLR_ENABLE);
            GICC.PMR.set(GICC::PMR_PRIORITY);
            GICC.CTLR.set(GICC::CTLR_ENABLE);
            barrier::dmb(barrier::SY);
        }
    }
}

#[kernel_module]
pub static GIC: GIC = GIC::new();

impl KernelModule for GIC {
    fn init(&'static mut self) -> anyhow::Result<()> {
        let devtree = SERVICE.get_device_tree().unwrap();
        let node = devtree
            .compatible("arm,cortex-a15-gic")
            .or_else(|| devtree.compatible("arm,gic-400"))
            .unwrap();
        interrupt::disable();
        let mut regs = node.regs().unwrap();
        let gicd_address = node.translate(regs.next().unwrap().start);
        let gicc_address = node.translate(regs.next().unwrap().start);
        // log!("GICD@{:?} GICC@{:?}", gicd_address, gicc_address);
        let gicd_page = SERVICE.map_device_page(Frame::new(gicd_address));
        let gicc_page = SERVICE.map_device_page(Frame::new(gicc_address));
        unsafe {
            *self.GICD.get() = gicd_page.start().as_mut_ptr();
            *self.GICC.get() = gicc_page.start().as_mut_ptr();
        }
        self.init_gic();
        SERVICE.set_interrupt_controller(self);
        Ok(())
    }
}

static mut IRQ_HANDLERS: [Option<IRQHandler>; IRQ_LINES] = {
    const IRQ_UNINIT: Option<IRQHandler> = None;
    [IRQ_UNINIT; IRQ_LINES]
};

impl InterruptController for GIC {
    fn get_active_irq(&self) -> usize {
        let gicc = GIC.gicc();
        let iar = gicc.IAR.get();
        self.iar.store(iar, Ordering::SeqCst);
        let irq = iar & GICC::IAR_INTERRUPT_ID__MASK;
        irq as _
    }

    fn enable_irq(&self, irq: usize) {
        unsafe {
            asm!("dsb SY");
            GIC.gicd().ISENABLER[irq / 32].set(1 << (irq & (32 - 1)));
            asm!("dmb SY");
        }
    }

    fn disable_irq(&self, _irq: usize) {
        unimplemented!()
    }

    fn notify_end_of_interrupt(&self) {
        GIC.gicc().EOIR.set(self.iar.load(Ordering::SeqCst));
    }

    fn get_irq_handler(&self, irq: usize) -> Option<&IRQHandler> {
        unsafe { IRQ_HANDLERS[irq].as_ref() }
    }

    fn set_irq_handler(&self, irq: usize, handler: IRQHandler) {
        unsafe {
            IRQ_HANDLERS[irq] = Some(handler);
        }
    }
}
