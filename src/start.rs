use crate::exception;
use cortex_a::{asm, regs::*};

#[no_mangle]
#[naked]
pub unsafe extern "C" fn _start() -> ! {
    // Halt non-promary processors
    asm! {"
            mrs     x0, mpidr_el1
            and     x0, x0, #3
            cbz     x0, 2f
        1:  wfe
            b       1b
        2:
    "};
    // Setup core 0 stack
    asm!("ldr x0, =_start; mov sp, x0");
    // Switch from EL2 -> EL1
    assert!(CurrentEL.get() == CurrentEL::EL::EL2.value);
    // Enable time counter registers
    CNTHCTL_EL2.set(CNTHCTL_EL2.get() | 0b11);
    CNTVOFF_EL2.set(0);
    // Set execution mode = AArch64
    HCR_EL2.write(HCR_EL2::RW::EL1IsAarch64);
    // Set EL1 interrupt vector
    VBAR_EL1.set(&exception::exception_handlers as *const _ as _);
    // Enable all co-processors
    asm!("msr cpacr_el1, $0"::"r"(0xfffffff));
    // Enable Debug+SError+IRQ+FIQ+EL1h
    SPSR_EL2.write(SPSR_EL2::D::Masked + SPSR_EL2::A::Masked + SPSR_EL2::I::Masked + SPSR_EL2::F::Masked + SPSR_EL2::M::EL1h);
    ELR_EL2.set(crate::kmain as *const () as u64); // EL1 PC after return from `eret`
    SP_EL1.set(0x80000); // EL1 stack
    asm::eret(); // Switch to EL1 -> kmain
}
