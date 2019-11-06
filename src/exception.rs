use crate::gpio::*;

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

#[no_mangle]
pub extern fn handle_exception() -> ! {
    loop {}
    unimplemented!();
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
    stp	x0,  x1,  [sp, #-16]!
    stp	x2,  x3,  [sp, #-16]!
    stp	x4,  x5,  [sp, #-16]!
    stp	x6,  x7,  [sp, #-16]!
    stp	x8,  x9,  [sp, #-16]!
    stp	x10, x11, [sp, #-16]!
    stp	x12, x13, [sp, #-16]!
    stp	x14, x15, [sp, #-16]!
.endm

.macro pop_all
    ldp	x0,  x1,  [sp], #16
	ldp	x2,  x3,  [sp], #16
	ldp	x4,  x5,  [sp], #16
	ldp	x6,  x7,  [sp], #16
	ldp	x8,  x9,  [sp], #16
	ldp	x10, x11, [sp], #16
	ldp	x12, x13, [sp], #16
    ldp	x14, x15, [sp], #16
.endm

.macro except_hang, exception_id
    .align 7
0:  wfi
    b 0b
.endm

.macro except, exception_id
    .align 7
    push_all
    bl handle_exception
    pop_all
    eret
.endm

.macro irq, exception_id
    .align 7
    push_all
    bl	handle_interrupt
    pop_all
    eret
.endm

    .balign 4096
exception_handlers:
    // Same exeception level, EL0
    except    #0 // Synchronous
    irq       #1 // IRQ
    except    #2 // FIQ
    except    #3 // SError
    // Same exeception level, ELx
    .align 9
    except    #0 // Synchronous
    irq       #1 // IRQ
    except    #2 // FIQ
    except    #3 // SError
    // Transit to upper exeception level, AArch64
    .align 9
    except    #0 // Synchronous
    irq       #1 // IRQ
    except    #2 // FIQ
    except    #3 // SError
    // Transit to upper exeception level, AArch32: Unreachable
    .align 9
    except    #0 // Synchronous
    irq       #1 // IRQ
    except    #2 // FIQ
    except    #3 // SError
"}