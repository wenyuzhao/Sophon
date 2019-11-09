use crate::mm::*;


/// Represents the archtectural context (i.e. registers)
#[repr(C)]
#[derive(Clone, Debug)]
pub struct Context {
    pub sp: *mut usize,
    pub x19: usize,
    pub x20: usize,
    pub x21: usize,
    pub x22: usize,
    pub x23: usize,
    pub x24: usize,
    pub x25: usize,
    pub x26: usize,
    pub x27: usize,
    pub x28: usize,
    pub x29: usize, // FP
    pub pc: *mut usize,  // x30
    pub p4: Frame,
}

impl Context {
    pub const fn empty() -> Self {
        Self {
            x19: 0, x20: 0, x21: 0, x22: 0, x23: 0, x24: 0,
            x25: 0, x26: 0, x27: 0, x28: 0, x29: 0,
            pc: 0 as _, sp: 0 as _,
            p4: Frame::ZERO,
        }
    }

    /// Create a new context with empty regs, given kernel stack,
    /// and current p4 table
    pub fn new(entry: *const extern fn() -> !, stack: *const u8) -> Self {
        Self {
            x19: 0, x20: 0, x21: 0, x22: 0, x23: 0, x24: 0,
            x25: 0, x26: 0, x27: 0, x28: 0, x29: 0,
            pc: unsafe { entry as _ },
            sp: unsafe { stack as _ },
            p4: Frame::ZERO,
        }
    }
}

impl Context {
    pub unsafe extern fn switch_to(&mut self, ctx: &Context) {
        // if self.p4 != ctx.p4 {
            debug!("Switch P4: {:?} -> {:?}", self.p4, ctx.p4);
            debug!("Switch SP: {:?} -> {:?}", self.sp, ctx.sp);
            // asm! {"
            //     msr	ttbr0_el1, $0
            //     tlbi vmalle1is
            //     DSB ISH
            //     isb
            // "::"r"(ctx.p4.start().as_usize())}
        // }
        switch_context(self, ctx, ctx.p4.start().as_usize())
    }
}

extern {
    fn switch_context(from: &mut Context, to: &Context, p4: usize);
    pub fn start_task();
}

global_asm! {"
.global switch_context

switch_context:
    // Store current registers

    mov x8, sp
    str x8, [x0], #8
    stp x19, x20, [x0], #16
    stp x21, x22, [x0], #16
    stp x23, x24, [x0], #16
    stp x25, x26, [x0], #16
    stp x27, x28, [x0], #16
    stp x29, x30, [x0], #16

    tlbi vmalle1is
    DSB ISH
    isb
    msr	ttbr0_el1, x2
    tlbi vmalle1is
    DSB ISH
    isb

    // Restore registers

    ldr x8, [x1], #8
    mov sp, x8
    ldp x19, x20, [x1], #16
    ldp x21, x22, [x1], #16
    ldp x23, x24, [x1], #16
    ldp x25, x26, [x1], #16
    ldp x27, x28, [x1], #16
    ldp x29, x30, [x1], #16 // FP, SP

    // Return
    ret
"}