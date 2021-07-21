use crate::arch::{Arch, ArchInterrupt, ArchInterruptController, TargetArch};
use crate::task::scheduler::AbstractScheduler;
use crate::utils::page::Frame;
use crate::utils::volatile::{PaddingForRange, Volatile, VolatileArrayForRange};
use crate::{
    arch::aarch64::INTERRUPT_CONTROLLER, boot_driver::BootDriver, task::scheduler::SCHEDULER,
};
use cortex_a::{asm::barrier, registers::*};
use fdt::node::FdtNode;
use spin::Mutex;
use tock_registers::interfaces::{Readable, Writeable};

const TIMER_INTERRUPT_FREQUENCY: usize = 1; // Hz

pub const IRQ_LINES: usize = 256;

#[repr(C)]
#[allow(non_snake_case)]
pub struct GICD {
    pub CTLR: Volatile<u32>,
    _0: PaddingForRange<{ 0x0004..0x0080 }>,
    pub IGROUPR: VolatileArrayForRange<u32, { 0x0080..0x00FC }>,
    _1: PaddingForRange<{ 0x00FC..0x0100 }>,
    pub ISENABLER: VolatileArrayForRange<u32, { 0x0100..0x017C }>,
    _2: PaddingForRange<{ 0x017C..0x0180 }>,
    pub ICENABLER: VolatileArrayForRange<u32, { 0x0180..0x01FC }>,
    _3: PaddingForRange<{ 0x01FC..0x0200 }>,
    pub ISPENDR: VolatileArrayForRange<u32, { 0x0200..0x027C }>,
    _4: PaddingForRange<{ 0x027C..0x0280 }>,
    pub ICPENDR: VolatileArrayForRange<u32, { 0x0280..0x02FC }>,
    _5: PaddingForRange<{ 0x02FC..0x0300 }>,
    pub ISACTIVER: VolatileArrayForRange<u32, { 0x0300..0x037C }>,
    _6: PaddingForRange<{ 0x037C..0x0380 }>,
    pub ICACTIVER: VolatileArrayForRange<u32, { 0x0380..0x03FC }>,
    _7: PaddingForRange<{ 0x03FC..0x0400 }>,
    pub IPRIORITYR: VolatileArrayForRange<u32, { 0x0400..0x07E0 }>,
    _8: PaddingForRange<{ 0x07E0..0x0800 }>,
    pub ITARGETSR: VolatileArrayForRange<u32, { 0x0800..0x0BE0 }>,
    _9: PaddingForRange<{ 0x0BE0..0x0C00 }>,
    pub ICFGR: VolatileArrayForRange<u32, { 0x0C00..0x0CF8 }>,
    _10: PaddingForRange<{ 0x0CF8..0x0F00 }>,
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
    _0: PaddingForRange<{ 0x0008..0x00C }>,
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
    GICD: Option<*mut GICD>,
    GICC: Option<*mut GICC>,
}

unsafe impl Send for GIC {}
unsafe impl Sync for GIC {}

impl GIC {
    const fn new() -> Self {
        Self {
            GICD: None,
            GICC: None,
        }
    }

    fn gicd(&self) -> &'static mut GICD {
        unsafe { &mut *self.GICD.unwrap() }
    }

    fn gicc(&self) -> &'static mut GICC {
        unsafe { &mut *self.GICC.unwrap() }
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

pub static mut GIC: GIC = GIC::new();

impl BootDriver for GIC {
    const COMPATIBLE: &'static [&'static str] = &["arm,cortex-a15-gic", "arm,gic-400"];
    fn init(&mut self, node: &FdtNode) {
        <TargetArch as Arch>::Interrupt::disable();

        let mut regs = node.reg().unwrap();
        let gicd_address = regs.next().unwrap().starting_address as usize;
        // if gicd_address & 0xff000000 == 0x7e000000 {
        //     gicd_address += 0xf0000000
        // }
        let gicc_address = regs.next().unwrap().starting_address as usize;
        // if gicc_address & 0xff000000 == 0x7e000000 {
        //     gicc_address += 0xf0000000
        // }
        log!("GICD@{:#x} GICC@{:#x}", gicd_address, gicc_address);
        let gicd_page = Self::map_device_page(Frame::new(gicd_address.into()));
        let gicc_page = Self::map_device_page(Frame::new(gicc_address.into()));
        self.GICD = Some(gicd_page.start().as_mut_ptr());
        self.GICC = Some(gicc_page.start().as_mut_ptr());
        self.init_gic();
        let irq = box GICInterruptController::new();
        irq.set_handler(
            crate::arch::InterruptId::Timer,
            Some(box |_, _, _, _, _, _| {
                // Update compare value
                let step = CNTFRQ_EL0.get() as u64 / TIMER_INTERRUPT_FREQUENCY as u64;
                CNTP_TVAL_EL0.set(step as u64);
                SCHEDULER.timer_tick();
                0
            }),
        );
        unsafe {
            INTERRUPT_CONTROLLER = Some(irq);
            super::super::exception::setup_vbar();
        }
    }
}

struct GICInterruptController {
    iar: Mutex<u32>,
}

impl GICInterruptController {
    pub fn new() -> Self {
        Self { iar: Mutex::new(0) }
    }
}

impl ArchInterruptController for GICInterruptController {
    fn start_timer(&self) {
        unsafe {
            asm!("dsb SY");
            let timer_irq = 16 + 14;
            GIC.gicd().ISENABLER[timer_irq / 32].set(1 << (timer_irq % 32));
            let n_cntfrq: usize = CNTFRQ_EL0.get() as _;
            assert!(n_cntfrq % TIMER_INTERRUPT_FREQUENCY == 0);
            let clock_ticks_per_timer_irq = n_cntfrq / TIMER_INTERRUPT_FREQUENCY;
            CNTP_TVAL_EL0.set(clock_ticks_per_timer_irq as u64);
            CNTP_CTL_EL0.set(1);
            asm!("dmb SY");
        }
    }

    fn get_active_irq(&self) -> usize {
        let gicc = unsafe { GIC.gicc() };
        let iar = gicc.IAR.get();
        *self.iar.lock() = iar;
        let irq = iar & GICC::IAR_INTERRUPT_ID__MASK;
        irq as _
    }

    fn notify_end_of_interrupt(&self) {
        unsafe { GIC.gicc().EOIR.set(*self.iar.lock()) };
    }
}
