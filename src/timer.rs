use crate::gpio::*;
use cortex_a::regs::*;
use crate::gic::*;
use crate::arch::*;

const TIMER_INTERRUPT_FREQUENCY: usize = 100; // Hz

// pub const ARM_TIMER_BASE: usize = 0xffff0000_40000000;

#[cfg(feature="device-raspi3")]
pub const ARM_TIMER_BASE: usize = 0xffff0000_40000000;
#[cfg(feature="device-raspi4")]
pub const ARM_TIMER_BASE: usize = 0xFFFF0000_FF800000;

const ARM_CONTROL_REGISTER: *mut u32 = (ARM_TIMER_BASE + 0x0) as _;
const ARM_INTERRUPT_ROUTING: *mut u32 = (ARM_TIMER_BASE + 0x24) as _;
const ARM_LOCAL_TIMER_CONTROL_AND_STATUS: *mut u32 = (ARM_TIMER_BASE + 0x34) as _;
const ARM_LOCAL_TIMER_CLEARL_AND_RELOAD: *mut u32 = (ARM_TIMER_BASE + 0x38) as _;
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

#[cfg(feature="qemu")]
pub fn init() {
    unsafe {
        // *ARM_INTERRUPT_ROUTING = 0b000;
        // *ARM_LOCAL_TIMER_CONTROL_AND_STATUS = (1 << 29) | (1 << 28) | 0x5000000;
        // *ARM_LOCAL_TIMER_CLEARL_AND_RELOAD = (1 << 30) | (1 << 31);
        // Enable nCNTPNSIRQ IRQ control
        // 64-bit Core timer increments by 1
        // *ARM_CONTROL_REGISTER &= !(1 << 9);
        // Set compare value
        // update_compare_value();
        // Enable timer
        // let c = CNTPCT_EL0.get();
        // asm!("msr CNTP_CVAL_EL0, $0":: "r"(c + 19200000));
        // CNTFRQ_EL0.set();

        let nCNTFRQ: usize = CNTFRQ_EL0.get() as _;
        println!("nCNTFRQ = {}", nCNTFRQ);
        assert!(nCNTFRQ % TIMER_INTERRUPT_FREQUENCY == 0);
        let clock_ticks_per_timer_irq = nCNTFRQ / TIMER_INTERRUPT_FREQUENCY;
        let nCNTPCT: usize = CNTPCT_EL0.get() as _;
        println!("nCNTPCT = {}", nCNTPCT);
        println!("clock_ticks_per_timer_irq = {}", clock_ticks_per_timer_irq);
        // loop {}
        asm!("msr CNTP_CVAL_EL0, $0" :: "r" (nCNTPCT + clock_ticks_per_timer_irq));
        CNTP_CTL_EL0.set(1);

        // CNTP_CTL_EL0.set(0b1);
        // CNTP_TVAL_EL0.set(1 << 24);
        *ARM_CORE_TIMER_INTERRUPT_CONTROL(0) = 1 << 1;
        // asm("msr CNTP_TVAL_EL0, %0" :: "r" (interval));
        // asm!("cpsie i");
    }
    Target::Interrupt::set_interrupt_handler(InterruptId::Timer, Some(handle_timer_irq));
}

static mut COUNT: u32 = 0;
const TIMER_CS: *mut u32 = (PERIPHERAL_BASE + 0x3000) as _;
const TIMER_CLO: *mut u32 = (PERIPHERAL_BASE + 0x3004) as _;
const TIMER_C0: *mut u32 = (PERIPHERAL_BASE + 0x300C) as _;
const TIMER_C1: *mut u32 = (PERIPHERAL_BASE + 0x3010) as _;
const TIMER_C3: *mut u32 = (PERIPHERAL_BASE + 0x3018) as _;

#[cfg(not(feature="qemu"))]
pub fn init() {
    println!("Timer init raspi4");
    unsafe {
        asm!("dsb SY":::"memory");

        let timer_irq = 16 + 14;
        GICD::get().ISENABLER[timer_irq / 32] = 1 << (timer_irq % 32);

        let nCNTFRQ: usize = CNTFRQ_EL0.get() as _;
        assert!(nCNTFRQ % TIMER_INTERRUPT_FREQUENCY == 0);
        let clock_ticks_per_timer_irq = nCNTFRQ / TIMER_INTERRUPT_FREQUENCY;
        let nCNTPCT: usize = CNTPCT_EL0.get() as _;
        asm!("msr CNTP_CVAL_EL0, $0" :: "r" (nCNTPCT + clock_ticks_per_timer_irq));
        CNTP_CTL_EL0.set(1);

        // COUNT = *TIMER_CLO;
        // COUNT += 2000000;
        // *TIMER_C3 = COUNT;
        // *TIMER_CS = 1 << 3;
        // *ENABLE_IRQS_1 = 1 << 3;
        // *ENABLE_BASIC_IRQS = 0xFF;
        asm!("dmb SY":::"memory");
    }
    Target::Interrupt::set_handler(InterruptId::Timer, Some(handle_timer_irq));
}

pub const ARMTIMER_VALUE: *mut u32     = (PERIPHERAL_BASE + 0xB404) as _;

#[inline]
fn update_compare_value() {
    debug_assert!(TIMER_INTERRUPT_FREQUENCY != 0);
    let freq = CNTFRQ_EL0.get() as u64;
    let step = freq / TIMER_INTERRUPT_FREQUENCY as u64;
    unsafe {
        asm!("msr cntp_cval_el0, $0":: "r"(CNTPCT_EL0.get() + step));
    }
}

#[inline]
pub fn pending_timer_irq() -> bool {
    ((unsafe { *ARM_CORE_TIMER_IRQ_SOURCE(0) }) & (1 << 1)) != 0
}

#[inline]
pub fn handle_timer_irq(_: usize, _: usize, _: usize, _: usize, _: usize, _: usize) -> isize {
    // println!("Timer iterrupt received");
    update_compare_value();
    crate::task::GLOBAL_TASK_SCHEDULER.timer_tick();
    0
}
