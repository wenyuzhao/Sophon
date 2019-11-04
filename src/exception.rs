
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

pub extern "C" fn exception_handler(kind: ExceptionKind, prev_el: ExceptionLevel, esr: usize, elr: usize, spsr: usize, far: usize) -> ! {
    debug!("Exception {:?}@{:?}: ESR={:x} ELR={:x} SPSR={:x} FAR={:x}", kind, prev_el, esr, elr, spsr, far);
    unimplemented!();
}

#[no_mangle]
#[naked]
pub unsafe extern "C" fn exception_entry(kind: ExceptionKind, prev_el: ExceptionLevel) -> ! {
    let esr:  usize; asm!("mrs $0, esr_el1": "=r"(esr));
    let elr:  usize; asm!("mrs $0, elr_el1": "=r"(elr));
    let spsr: usize; asm!("mrs $0, spsr_el1": "=r"(spsr));
    let far:  usize; asm!("mrs $0, far_el1": "=r"(far));
    exception_handler(kind, prev_el, esr, elr, spsr, far);
}

extern {
    pub static exception_handlers: u8;
}

// Exception handlers table
global_asm! {"
.global exception_handlers

.macro except, exception_id
    .align 7
    mov       x0, \\exception_id
    mrs       x1, CurrentEL
    and       x1, x1, #0b1100
    lsr	      x1, x1, #2
    b         exception_entry
.endm

    .align 11
exception_handlers:
    // Same exeception level, EL0
    except    #0 // Synchronous
    except    #1 // IRQ
    except    #2 // FIQ
    except    #3 // SError
    // Same exeception level, ELx
    .align 9
    except    #0 // Synchronous
    except    #1 // IRQ
    except    #2 // FIQ
    except    #3 // SError
    // Transit to upper exeception level, AArch64
    .align 9
    except    #0 // Synchronous
    except    #1 // IRQ
    except    #2 // FIQ
    except    #3 // SError
    // Transit to upper exeception level, AArch32: Unreachable
"}