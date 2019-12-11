use crate::gpio::*;
use cortex_a::regs::*;
use crate::gic::*;

pub const IRQ_BASIC_PENDING:  *mut u32 = (PERIPHERAL_BASE + 0xB200) as _;
pub const IRQ_PENDING_1:      *mut u32 = (PERIPHERAL_BASE + 0xB204) as _;
pub const IRQ_PENDING_2:      *mut u32 = (PERIPHERAL_BASE + 0xB208) as _;
pub const FIQ_CONTROL:        *mut u32 = (PERIPHERAL_BASE + 0xB20C) as _;
pub const ENABLE_IRQS_1:      *mut u32 = (PERIPHERAL_BASE + 0xB210) as _;
pub const ENABLE_IRQS_2:      *mut u32 = (PERIPHERAL_BASE + 0xB214) as _;
pub const ENABLE_BASIC_IRQS:  *mut u32 = (PERIPHERAL_BASE + 0xB218) as _;
pub const DISABLE_IRQS_1:     *mut u32 = (PERIPHERAL_BASE + 0xB21C) as _;
pub const DISABLE_IRQS_2:     *mut u32 = (PERIPHERAL_BASE + 0xB220) as _;
pub const DISABLE_BASIC_IRQS: *mut u32 = (PERIPHERAL_BASE + 0xB224) as _;



pub fn initialize() {
    if cfg!(feature="qemu") {
        return
    }
    let GICD = GICD::get();
    let GICC = GICC::get();
    unsafe {
        asm!("dsb SY":::"memory");
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
        asm!("dmb SY":::"memory");
    }
}

pub fn is_enabled() -> bool {
    unsafe {
        let daif: usize;
        asm!("mrs $0, DAIF":"=r"(daif));
        daif & (1 << 7) == 0
    }
}

pub fn enable() {
    unsafe {
        
        // asm!("dsb":::"memory");
        asm!("msr daifclr, #2");
        
        // asm!("dmb":::"memory");
    };
}

pub fn disable() {
    unsafe { asm!("msr daifset, #2") };
}

pub fn uninterruptable<R, F: FnOnce() -> R>(f: F) -> R {
    let enabled = is_enabled();
    if enabled {
        disable();
    }
    let ret = f();
    if enabled {
        enable();
    }
    ret
}

#[cfg(feature="device-raspi4")]
#[no_mangle]
pub extern fn handle_interrupt() {
    let GICC = GICC::get();
    let iar = GICC.IAR;
    let irq = iar & GICC::IAR_INTERRUPT_ID__MASK;
    if irq < 256 {
        // boot_log!("=== Int received ===");
        // println!("GICC_IAR = {}, IRQ = {}", iar, irq);

        if irq == 30 {
            // FIXME: End of Interrupt ??? here ???
            GICC.EOIR = iar;
            crate::timer::handle_timer_irq();
            return;
        }

        GICC.EOIR = iar;
    }
    // boot_log!("=== Int received ===");
    // println!("GICC_IAR = {}", IAR);
    // println!("nIRQ = {}", irq);
    // println!("TIMER_CS = 0b{:b}", unsafe { *crate::timer::TIMER_CS });
    
    // unsafe {
    //     println!("IRQ_BASIC_PENDING = 0b{:b}", *IRQ_BASIC_PENDING);
    //     println!("IRQ_PENDING_1 = 0b{:b}", *IRQ_PENDING_1);
    //     println!("IRQ_PENDING_2 = 0b{:b}", *IRQ_PENDING_2);
    // }

    // if irq == 30 {
    //     crate::timer::handle_timer_irq();
    //     return
    // }

    // // if 
    
    // if crate::timer::pending_timer_irq() {
    //     crate::timer::handle_timer_irq();
    // } else {
    //     println!("Unknown IRQ");
    //     loop {}
    // }
}

#[cfg(all(feature="device-raspi3"))]
#[no_mangle]
pub extern fn handle_interrupt() {
    if !cfg!(feature="qemu") {
        unimplemented!();
    }
    // println!("=== Int received ===");

    if crate::timer::pending_timer_irq() {
        crate::timer::handle_timer_irq();
    } else {
        println!("Unknown IRQ");
        loop {}
    }
}
