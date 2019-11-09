use crate::gpio::*;
use cortex_a::regs::*;

#[repr(usize)]
#[derive(Debug)]
pub enum ExceptionLevel {
    EL0 = 0,
    EL1 = 1,
    EL2 = 2,
}

#[repr(usize)]
#[derive(Debug)]
pub enum ExceptionKind {
    Synchronous = 0,
    IRQ = 1,
    FIQ = 2,
    SError = 3,
}

#[repr(C)]
pub struct ExceptionFrame {
    pub elr_el1: usize,
    pub spsr_el1: usize,
    pub x30: usize,
    pub sp_el0: usize,
    pub x28: usize,
    pub x29: usize,
    pub x26: usize,
    pub x27: usize,
    pub x24: usize,
    pub x25: usize,
    pub x22: usize,
    pub x23: usize,
    pub x20: usize,
    pub x21: usize,
    pub x18: usize,
    pub x19: usize,
    pub x16: usize,
    pub x17: usize,
    pub x14: usize,
    pub x15: usize,
    pub x12: usize,
    pub x13: usize,
    pub x10: usize,
    pub x11: usize,
    pub x8: usize,
    pub x9: usize,
    pub x6: usize,
    pub x7: usize,
    pub x4: usize,
    pub x5: usize,
    pub x2: usize,
    pub x3: usize,
    pub x0: usize,
    pub x1: usize,
}

#[no_mangle]
pub unsafe extern fn handle_exception(exception_frame: *mut ExceptionFrame) {
    let esr_el1: usize;
    asm!("mrs $0, esr_el1":"=r"(esr_el1));
    debug!("exception at frame {:?}", exception_frame);
    if (esr_el1 >> 26) == 0x15 {
        crate::syscall::handle_syscall(&mut *exception_frame);
    } else {
        unimplemented!();
    }
}

pub unsafe extern fn exit_from_exception2() {
    loop {}
}

extern {
    pub static exception_handlers: u8;
    pub fn exit_from_exception() -> !;
}

// FIXME: We may need to switch stack after enter an exception,
//        to avoid stack overflow.
// Exception handlers table
global_asm! {"
.global exception_handlers

.macro push_all
    stp x0,  x1,  [sp, #-16]!
    stp x2,  x3,  [sp, #-16]!
    stp x4,  x5,  [sp, #-16]!
    stp x6,  x7,  [sp, #-16]!
    stp x8,  x9,  [sp, #-16]!
    stp x10, x11, [sp, #-16]!
    stp x12, x13, [sp, #-16]!
    stp x14, x15, [sp, #-16]!
    stp x16, x17, [sp, #-16]!
    stp x18, x19, [sp, #-16]!
    stp x20, x21, [sp, #-16]!
    stp x22, x23, [sp, #-16]!
    stp x24, x25, [sp, #-16]!
    stp x26, x27, [sp, #-16]!
    stp x28, x29, [sp, #-16]!
    mrs	x21, sp_el0
    mrs x22, elr_el1
    mrs x23, spsr_el1
    stp x30, x21, [sp, #-16]!
    stp x22, x23, [sp, #-16]!
.endm

.macro pop_all
    ldp x22, x23, [sp], #16
    ldp x30, x21, [sp], #16
    msr	sp_el0, x21
    msr elr_el1, x22  
    msr spsr_el1, x23
    ldp x28, x29, [sp], #16
    ldp x26, x27, [sp], #16
    ldp x24, x25, [sp], #16
    ldp x22, x23, [sp], #16
    ldp x20, x21, [sp], #16
    ldp x18, x19, [sp], #16
    ldp x16, x17, [sp], #16
    ldp x14, x15, [sp], #16
    ldp x12, x13, [sp], #16
    ldp x10, x11, [sp], #16
    ldp x8,  x9,  [sp], #16
    ldp x6,  x7,  [sp], #16
    ldp x4,  x5,  [sp], #16
    ldp x2,  x3,  [sp], #16
    ldp x0,  x1,  [sp], #16
.endm

.macro except_hang, exception_id
    .align 7
0:  wfi
    b 0b
.endm

except:
    push_all
    mov x0, sp
    bl handle_exception
    pop_all
    eret

irq:
    push_all
    bl handle_interrupt
    pop_all
    eret

    .balign 4096
exception_handlers:
    // Same exeception level, EL0
    .align 9; b except
    .align 7; b irq
    .align 7; b except
    .align 7; b except
    // Same exeception level, ELx
    .align 9; b except
    .align 7; b irq
    .align 7; b except
    .align 7; b except
    // Transit to upper exeception level, AArch64
    .align 9; b except
    .align 7; b irq
    .align 7; b except
    .align 7; b except
    // Transit to upper exeception level, AArch32: Unreachable
    .align 9; b except
    .align 7; b irq
    .align 7; b except
    .align 7; b except

.global exit_from_exception
exit_from_exception:
    msr	daifset, #2
    pop_all
    eret
"}