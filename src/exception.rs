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

#[repr(u32)]
#[derive(Debug)]
pub enum ExceptionClass {
    SVCAArch64 = 0b010101,
    DataAbortLowerEL = 0b100100,
    DataAbortHigherEL = 0b100101,
}

#[repr(C)]
pub struct ExceptionFrame {
    pub q30: u128,
    pub q31: u128,
    pub q28: u128,
    pub q29: u128,
    pub q26: u128,
    pub q27: u128,
    pub q24: u128,
    pub q25: u128,
    pub q22: u128,
    pub q23: u128,
    pub q20: u128,
    pub q21: u128,
    pub q18: u128,
    pub q19: u128,
    pub q16: u128,
    pub q17: u128,
    pub q14: u128,
    pub q15: u128,
    pub q12: u128,
    pub q13: u128,
    pub q10: u128,
    pub q11: u128,
    pub q8: u128,
    pub q9: u128,
    pub q6: u128,
    pub q7: u128,
    pub q4: u128,
    pub q5: u128,
    pub q2: u128,
    pub q3: u128,
    pub q0: u128,
    pub q1: u128,

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

unsafe fn get_exception_class() -> ExceptionClass {
    let esr_el1: u32;
    asm!("mrs $0, esr_el1":"=r"(esr_el1));
    ::core::mem::transmute(esr_el1 >> 26)
}

#[no_mangle]
pub unsafe extern fn handle_exception(exception_frame: *mut ExceptionFrame) -> isize {
    println!("Exception received {:?}", get_exception_class());
    println!("{:x}", SPSR_EL1.get());
    println!("{:x}", SP.get());
    match get_exception_class() {
        ExceptionClass::SVCAArch64 => crate::syscall::handle_syscall(&mut *exception_frame),
        ExceptionClass::DataAbortLowerEL | ExceptionClass::DataAbortHigherEL => {
            let far: usize;
            asm!("mrs $0, far_el1":"=r"(far));
            let elr: usize;
            asm!("mrs $0, elr_el1":"=r"(elr));
            println!("Data Abort {:?} {:?}", far as *mut (), elr as *mut ());
            println!("Data Abort {:?}, {:?}", far as *mut (), crate::task::Task::current().unwrap().id());
            crate::mm::handle_user_pagefault(far.into());
        },
        v => panic!("Unknown exception 0b{:b}", v as u32),
    }
    0
}


extern {
    pub static exception_handlers: u8;
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
    stp q0,  q1,  [sp, #-32]!
    stp q2,  q3,  [sp, #-32]!
    stp q4,  q5,  [sp, #-32]!
    stp q6,  q7,  [sp, #-32]!
    stp q8,  q9,  [sp, #-32]!
    stp q10, q11, [sp, #-32]!
    stp q12, q13, [sp, #-32]!
    stp q14, q15, [sp, #-32]!
    stp q16, q17, [sp, #-32]!
    stp q18, q19, [sp, #-32]!
    stp q20, q21, [sp, #-32]!
    stp q22, q23, [sp, #-32]!
    stp q24, q25, [sp, #-32]!
    stp q26, q27, [sp, #-32]!
    stp q28, q29, [sp, #-32]!
    stp q30, q31, [sp, #-32]!
.endm

.macro pop_all
    ldp q30, q31, [sp], #32
    ldp q28, q29, [sp], #32
    ldp q26, q27, [sp], #32
    ldp q24, q25, [sp], #32
    ldp q22, q23, [sp], #32
    ldp q20, q21, [sp], #32
    ldp q18, q19, [sp], #32
    ldp q16, q17, [sp], #32
    ldp q14, q15, [sp], #32
    ldp q12, q13, [sp], #32
    ldp q10, q11, [sp], #32
    ldp q8,  q9,  [sp], #32
    ldp q6,  q7,  [sp], #32
    ldp q4,  q5,  [sp], #32
    ldp q2,  q3,  [sp], #32
    ldp q0,  q1,  [sp], #32
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
"}