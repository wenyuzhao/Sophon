use crate::gpio::*;
use cortex_a::regs::*;

const TIMER_INTERRUPT_FREQUENCY: usize = 100; // Hz

pub const ARM_TIMER_BASE: usize = 0xffff0000_40000000;
const ARM_CONTROL_REGISTER: *mut u32 = (ARM_TIMER_BASE + 0x0) as _;
const ARM_CORE_TIMER_INTERRUPT_CONTROL_BASE: usize = ARM_TIMER_BASE + 0x40;
const ARM_CORE_TIMER_IRQ_SOURCE_BASE: usize = ARM_TIMER_BASE + 0x60;

#[allow(non_snake_case)]
const fn ARM_CORE_TIMER_INTERRUPT_CONTROL(core: u8) -> *mut u32 {
    // 0x40, 0x44, 0x48, 0x4c: Core 0~3 Timers interrupt control
    (ARM_CORE_TIMER_INTERRUPT_CONTROL_BASE + 0x4 * (core as usize)) as _
}

#[allow(non_snake_case)]
const fn ARM_CORE_TIMER_IRQ_SOURCE(core: u8) -> *mut u32 {
    (ARM_CORE_TIMER_IRQ_SOURCE_BASE + 0x4 * (core as usize)) as _
}

pub fn init() {
    unsafe {
        // 64-bit Core timer increments by 1
        *ARM_CONTROL_REGISTER &= !(1 << 9);
        // Enable nCNTPNSIRQ IRQ control
        *ARM_CORE_TIMER_INTERRUPT_CONTROL(0) = 1 << 1;
        // Set compare value
        update_compare_value();
        // Enable timer
        CNTP_CTL_EL0.set(1);
    }
}

#[inline]
fn update_compare_value() {
    debug_assert!(TIMER_INTERRUPT_FREQUENCY != 0);
    let freq = CNTFRQ_EL0.get() as u64;
    let step = freq / TIMER_INTERRUPT_FREQUENCY as u64;
    unsafe {
        asm!("msr cntp_cval_el0, $0":: "r"(CNTPCT_EL0.get() + step));
    }
}

fn timer_count() -> usize {
    CNTPCT_EL0.get() as _
}

#[inline]
pub fn pending_timer_irq() -> bool {
    ((unsafe { *ARM_CORE_TIMER_IRQ_SOURCE(0) }) & (1 << 1)) != 0
}

#[inline]
pub fn handle_timer_irq() {
    // println!("Timer iterrupt received, count = {}", timer_count());
    update_compare_value();

    crate::task::GLOBAL_TASK_SCHEDULER.timer_tick();
}
