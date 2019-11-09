use crate::gpio::*;
use cortex_a::regs::*;


const IRQ_BASIC_PENDING:  *mut u32 = (PERIPHERAL_BASE + 0xB200) as _;
const IRQ_PENDING_1:      *mut u32 = (PERIPHERAL_BASE + 0xB204) as _;
const IRQ_PENDING_2:      *mut u32 = (PERIPHERAL_BASE + 0xB208) as _;
const FIQ_CONTROL:        *mut u32 = (PERIPHERAL_BASE + 0xB20C) as _;
const ENABLE_IRQS_1:      *mut u32 = (PERIPHERAL_BASE + 0xB210) as _;
const ENABLE_IRQS_2:      *mut u32 = (PERIPHERAL_BASE + 0xB214) as _;
const ENABLE_BASIC_IRQS:  *mut u32 = (PERIPHERAL_BASE + 0xB218) as _;
const DISABLE_IRQS_1:     *mut u32 = (PERIPHERAL_BASE + 0xB21C) as _;
const DISABLE_IRQS_2:     *mut u32 = (PERIPHERAL_BASE + 0xB220) as _;
const DISABLE_BASIC_IRQS: *mut u32 = (PERIPHERAL_BASE + 0xB224) as _;

pub fn is_enabled() -> bool {
    unsafe {
        let daif: usize;
        asm!("mrs $0, DAIF":"=r"(daif));
        daif & (1 << 7) == 0
    }
}

pub fn enable() {
    unsafe { asm!("msr daifclr, #2") };
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

#[no_mangle]
pub extern fn handle_interrupt() {
    if crate::timer::pending_timer_irq() {
        crate::timer::handle_timer_irq();
    } else {
        println!("Unknown IRQ");
        loop {}
    }
}