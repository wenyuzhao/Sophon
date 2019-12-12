use crate::arch::*;
use super::gic::*;
use cortex_a::barrier;
use super::exception::*;

pub struct InterruptController;

static mut INTERRUPT_HANDLERS: [Option<InterruptHandler>; 256] = [None; 256];

pub fn handle_interrupt(kind: InterruptId, exception_frame: &mut ExceptionFrame) -> isize {
    // println!("<int> {:?}", kind);
    if let Some(handler) = unsafe { INTERRUPT_HANDLERS[kind as usize] } {
        handler(
            exception_frame.x0, exception_frame.x1, exception_frame.x2,
            exception_frame.x3, exception_frame.x4, exception_frame.x5,
        );
        0
        // result
        // exception_frame.x0 = unsafe { ::core::mem::transmute(result) };
    } else {
        println!("Interrupt<{:?}> has no handler!", kind);
        0
    }
}

impl AbstractInterruptController for InterruptController {
    fn init() {
        if cfg!(feature="qemu") || cfg!(feature="device-raspi3") {
            return
        }
        let GICD = GICD::get();
        let GICC = GICC::get();
        unsafe { barrier::dsb(barrier::SY) };
        // Disable all interrupts
        GICD.CTLR = GICD::CTLR_DISABLE;
        for n in 0..(IRQ_LINES / 32) {
            GICD.ICENABLER[n] = !0;
            GICD.ICPENDR[n] = !0;
            GICD.ICACTIVER[n] = !0;
        }
        // Connect interrupts to core#0
        for n in 0..(IRQ_LINES / 4) {
            GICD.IPRIORITYR[n] = GICD::IPRIORITYRAULT | GICD::IPRIORITYRAULT << 8 | GICD::IPRIORITYRAULT << 16 | GICD::IPRIORITYRAULT << 24;
            GICD.ITARGETSR[n] = GICD::ITARGETSR_CORE0 | GICD::ITARGETSR_CORE0 << 8 | GICD::ITARGETSR_CORE0 << 16 | GICD::ITARGETSR_CORE0 << 24;
        }
        // set all interrupts to level triggered
        for n in 0..(IRQ_LINES / 16) {
            GICD.ICFGR[n] = 0;
        }
        // Enable GIC
        GICD.CTLR = GICD::CTLR_ENABLE;
        GICC.PMR = GICC::PMR_PRIORITY;
        GICC.CTLR = GICC::CTLR_ENABLE;
        unsafe { barrier::dmb(barrier::SY) };
    }
    
    fn is_enabled() -> bool {
        unsafe {
            let daif: usize;
            asm!("mrs $0, DAIF":"=r"(daif));
            daif & (1 << 7) == 0
        }
    }
    
    fn enable() {
        unsafe { asm!("msr daifclr, #2") };
    }
    
    fn disable() {
        unsafe { asm!("msr daifset, #2") };
    }

    fn set_handler(id: InterruptId, handler: Option<InterruptHandler>) {
        unsafe {
            INTERRUPT_HANDLERS[id as usize] = handler;
        }
    }
}
