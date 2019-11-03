.section ".text._start"

.global _start

_start:
    mrs     x1, mpidr_el1
    and     x1, x1, #3
    cbz     x1, 2f
// Halt non-primary cores
1:
    wfe
    b       1b
// We are in core 0
// Switch to execution level 1
2:
    // Are we already in EL1?
    mrs     x0, CurrentEL
    and     x0, x0, #0b1100
    cmp     x0, #0b0100
    beq     3f
    // Switch EL2 -> EL1
    // FIXME: what if we in EL3?
    ldr     x1, =_start
    msr     sp_el1, x1
    // Enable CNTP for EL1
    mrs     x0, cnthctl_el2
    orr     x0, x0, #3
    msr     cnthctl_el2, x0
    msr     cntvoff_el2, xzr
    // Enable AArch64 for EL1
    mov     x0, #(1 << 31)      // AArch64
    orr     x0, x0, #(1 << 1)   // SWIO hardwired on Pi3
    msr     hcr_el2, x0
    // Set up exception handlers for EL1
    ldr     x2, =_execptoin_handlers
    msr     vbar_el1, x2
    // Enable access to coprocessors 0~13 for EL1
    ldr x0, =0xfffffff
    msr cpacr_el1,x0
    // Switch to EL1
    mov     x2, #0x3c4
    msr     spsr_el2, x2
    adr     x2, 3f
    msr     elr_el2, x2
    eret
// We are in EL1 now
3:  
    ldr     x1, =_start
    mov     sp, x1
    // Jump to rust code
    bl      kmain
    b       1b

// Exception handlers table
    .align 11
_execptoin_handlers:
    // synchronous
    .align  7
    mov     x0, #0
    mrs     x1, esr_el1
    mrs     x2, elr_el1
    mrs     x3, spsr_el1
    mrs     x4, far_el1
    b       exc_handler
    // IRQ
    .align  7
    mov     x0, #1
    mrs     x1, esr_el1
    mrs     x2, elr_el1
    mrs     x3, spsr_el1
    mrs     x4, far_el1
    b       exc_handler
    // FIQ
    .align  7
    mov     x0, #2
    mrs     x1, esr_el1
    mrs     x2, elr_el1
    mrs     x3, spsr_el1
    mrs     x4, far_el1
    b       exc_handler
    // SError
    .align  7
    mov     x0, #3
    mrs     x1, esr_el1
    mrs     x2, elr_el1
    mrs     x3, spsr_el1
    mrs     x4, far_el1
    b       exc_handler
