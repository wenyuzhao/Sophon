#![no_std]
#![feature(asm)]

#[inline]
#[cfg(target_arch = "aarch64")]
pub fn enable() {
    unsafe { asm!("msr daifclr, #2") };
}

#[inline]
#[cfg(target_arch = "aarch64")]
pub fn disable() {
    unsafe { asm!("msr daifset, #2") };
}

#[inline]
#[cfg(target_arch = "aarch64")]
pub fn is_enabled() -> bool {
    unsafe {
        let daif: usize;
        asm!("mrs {}, DAIF", out(reg) daif);
        daif & (1 << 7) == 0
    }
}

#[inline]
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