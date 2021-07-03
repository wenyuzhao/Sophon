use core::{intrinsics::volatile_store, mem, slice};

use crate::scheduler::AbstractScheduler;
use crate::{
    arch::{aarch64::INTERRUPT_CONTROLLER, ArchInterrupt},
    boot_driver::BootDriver,
    scheduler::SCHEDULER,
};
use cortex_a::{barrier, regs::*};
use device_tree::Node;
use spin::Lazy;

// pub const ARM_GICD_BASE: usize = super::timer::ARM_TIMER_BASE;
// pub const ARM_GICC_BASE: usize = super::timer::ARM_TIMER_BASE + 0x10000;
const TIMER_INTERRUPT_FREQUENCY: usize = 1; // Hz

pub const IRQ_LINES: usize = 256;

macro_rules! u32_array {
    ($start: literal - $end: literal) => {
        [u32; ($end - $start + 4) / 4]
    };
}

macro_rules! pad {
    ($curr_end: literal - $next_start: literal) => {
        [u8; $next_start - $curr_end - 4]
    };
    (bytes: $bytes: literal) => {
        [u8; $bytes]
    };
}

#[repr(C)]
#[allow(non_snake_case)]
pub struct GICD {
    pub CTLR: u32,
    /* 0x0000 */ _0: pad![0x0000 - 0x0080],
    pub IGROUPR: u32_array![0x0080 - 0x00F8],
    _1: pad![bytes: 1],
    pub ISENABLER: u32_array![0x0100 - 0x0178],
    _2: pad![bytes: 1],
    pub ICENABLER: u32_array![0x0180 - 0x01F8],
    _3: pad![bytes: 1],
    pub ISPENDR: u32_array![0x0200 - 0x0278],
    _4: pad![bytes: 1],
    pub ICPENDR: u32_array![0x0280 - 0x02F8],
    _5: pad![bytes: 1],
    pub ISACTIVER: u32_array![0x0300 - 0x0378],
    _6: pad![bytes: 1],
    pub ICACTIVER: u32_array![0x0380 - 0x03F8],
    _7: pad![bytes: 1],
    pub IPRIORITYR: u32_array![0x0400 - 0x07DC],
    _8: pad![0x07DC - 0x0800],
    pub ITARGETSR: u32_array![0x0800 - 0x0BDC],
    _9: pad![0x0BDC - 0x0C00],
    pub ICFGR: u32_array![0x0C00 - 0x0CF4],
    _10: pad![0x0CF4 - 0x0F00],
    pub SGIR: u32, /* 0x0F00 */
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
    pub CTLR: u32, // 0x000
    pub PMR: u32,  // 0x004;
    _0: pad![0x004 - 0x00C],
    pub IAR: u32,  // 0x00C
    pub EOIR: u32, // 0x010
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
    GICD: Option<*mut GICD>,
    GICC: Option<*mut GICC>,
}

unsafe impl Send for GIC {}
unsafe impl Sync for GIC {}

impl GIC {
    pub fn gicd(&self) -> &'static mut GICD {
        unsafe { &mut *self.GICD.unwrap() }
    }
    pub fn gicc(&self) -> &'static mut GICC {
        unsafe { &mut *self.GICC.unwrap() }
    }

    pub fn init(&self) {
        #[allow(non_snake_case)]
        let (GICD, GICC) = (self.gicd(), self.gicc());
        unsafe { barrier::dsb(barrier::SY) };
        unsafe {
            // Disable all interrupts
            volatile_store(&mut GICD.CTLR, GICD::CTLR_DISABLE);
            for n in 0..(IRQ_LINES / 32) {
                volatile_store(&mut GICD.ICENABLER[n], !0);
                volatile_store(&mut GICD.ICPENDR[n], !0);
                volatile_store(&mut GICD.ICACTIVER[n], !0);
            }
            // Connect interrupts to core#0
            for n in 0..(IRQ_LINES / 4) {
                volatile_store(
                    &mut GICD.IPRIORITYR[n],
                    GICD::IPRIORITYRAULT
                        | GICD::IPRIORITYRAULT << 8
                        | GICD::IPRIORITYRAULT << 16
                        | GICD::IPRIORITYRAULT << 24,
                );
                volatile_store(
                    &mut GICD.ITARGETSR[n],
                    GICD::ITARGETSR_CORE0
                        | GICD::ITARGETSR_CORE0 << 8
                        | GICD::ITARGETSR_CORE0 << 16
                        | GICD::ITARGETSR_CORE0 << 24,
                );
            }
            // set all interrupts to level triggered
            for n in 0..(IRQ_LINES / 16) {
                volatile_store(&mut GICD.ICFGR[n], 0);
            }
            // Enable GIC
            volatile_store(&mut GICD.CTLR, GICD::CTLR_ENABLE);
            volatile_store(&mut GICC.PMR, GICC::PMR_PRIORITY);
            volatile_store(&mut GICC.CTLR, GICC::CTLR_ENABLE);
            barrier::dmb(barrier::SY);
        }
    }
}

pub static GIC: Lazy<GIC> = Lazy::new(|| GIC {
    GICD: None,
    GICC: None,
});

impl BootDriver for GIC {
    const COMPATIBLE: &'static str = "arm,cortex-a15-gic";
    fn init(&mut self, node: &Node) {
        unsafe { asm!("msr daifset, #2") };

        assert!(node.prop_raw("#size-cells").unwrap() == &[0u8, 0, 0, 2]);
        assert!(node.prop_raw("#address-cells").unwrap() == &[0u8, 0, 0, 2]);
        // reg.
        let reg = node.prop_raw("reg").unwrap();
        // log!("reg bytes: {:?}", reg);
        let len = reg.len() / 4;
        let data = unsafe { slice::from_raw_parts(reg.as_ptr() as *const u32, len) };
        log!("reg: {:?}", data);
        let gicd_address = ((u32::from_be(data[0]) as u64) << 32) | (u32::from_be(data[1]) as u64);
        // let gicd_size = ((u32::from_be(data[2]) as u64) << 32) | (u32::from_be(data[3]) as u64);
        let gicc_address = ((u32::from_be(data[4]) as u64) << 32) | (u32::from_be(data[5]) as u64);
        // let gicc_size = ((u32::from_be(data[6]) as u64) << 32) | (u32::from_be(data[7]) as u64);
        log!("GICD@{:#x} GICC@{:#x}", gicd_address, gicc_address);
        self.GICD = Some(unsafe { mem::transmute(gicd_address) });
        self.GICC = Some(unsafe { mem::transmute(gicc_address) });
        let irq = box GICInterruptController::new();
        irq.set_handler(
            crate::arch::InterruptId::Timer,
            Some(box |_, _, _, _, _, _| {
                // Update compare value
                let step = CNTFRQ_EL0.get() as u64 / TIMER_INTERRUPT_FREQUENCY as u64;
                unsafe {
                    asm!("msr cntp_cval_el0, {}", in(reg) CNTPCT_EL0.get() + step);
                }
                SCHEDULER.timer_tick();
                0
            }),
        );
        unsafe {
            INTERRUPT_CONTROLLER = Some(irq);
        }

        log!("Starting timer...");

        unsafe {
            asm!("dsb SY");
            let timer_irq = 16 + 14;
            self.gicd().ISENABLER[timer_irq / 32] = 1 << (timer_irq % 32);
            let n_cntfrq: usize = CNTFRQ_EL0.get() as _;
            assert!(n_cntfrq % TIMER_INTERRUPT_FREQUENCY == 0);
            let clock_ticks_per_timer_irq = n_cntfrq / TIMER_INTERRUPT_FREQUENCY;
            let n_cntpct: usize = CNTPCT_EL0.get() as _;
            asm!("msr CNTP_CVAL_EL0, {}", in(reg) n_cntpct + clock_ticks_per_timer_irq);
            CNTP_CTL_EL0.set(1);
            asm!("dmb SY");
        }

        log!("Timer started");

        // unsafe { llvm_asm!("msr daifclr, #2") };
        // unsafe { log!("Int enabled: {}", INTERRUPT_CONTROLLER.as_ref().unwrap().is_enabled()); }

        unsafe {
            super::super::exception::setup_vbar();
        }

        // unsafe { INTERRUPT_CONTROLLER.as_ref().unwrap().disable(); }
        // unsafe { log!("Int enabled: {}", INTERRUPT_CONTROLLER.as_ref().unwrap().is_enabled()); }
    }
}

struct GICInterruptController {}

impl GICInterruptController {
    pub fn new() -> Self {
        Self {}
    }
}

impl ArchInterrupt for GICInterruptController {
    fn is_enabled(&self) -> bool {
        unsafe {
            let daif: usize;
            asm!("mrs {}, DAIF", out(reg) daif);
            daif & (1 << 7) == 0
        }
    }

    fn enable(&self) {
        unsafe { asm!("msr daifclr, #2") };
    }

    fn disable(&self) {
        unsafe { asm!("msr daifset, #2") };
    }
}
