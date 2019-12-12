use super::*;
use crate::mm::paging;
use cortex_a::{asm, regs::*, barrier};

#[inline(always)]
#[naked]
pub unsafe fn _start() -> ! {
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
    asm!("mov sp, $0"::"r"(0x80000));
    crate::debug_boot::UART::init();
    assert!(CurrentEL.get() == CurrentEL::EL::EL2.value);
    crate::debug_boot::log("[boot...]");
    CNTHCTL_EL2.write(CNTHCTL_EL2::EL1PCEN::SET + CNTHCTL_EL2::EL1PCTEN::SET);
    CNTVOFF_EL2.set(0);
    // Switch to EL1
    SCTLR_EL1.set((3 << 28) | (3 << 22) | (1 << 20) | (1 << 11)); // Disable MMU
    HCR_EL2.write(HCR_EL2::RW::EL1IsAarch64); // Set execution mode = AArch64
    SPSR_EL2.write(SPSR_EL2::D::Masked + SPSR_EL2::A::Masked + SPSR_EL2::I::Masked + SPSR_EL2::F::Masked + SPSR_EL2::M::EL1h);
    ELR_EL2.set(_start_el1 as *const () as u64); // EL1 PC after return from `eret`
    SP_EL1.set(0x80000); // EL1 stack
    asm::eret();
}


extern {
    static mut __bss_start: usize;
    static mut __bss_end: usize;
}

unsafe fn zero_bss() {
    let start = (&mut __bss_start as *mut usize as usize & 0x0000ffff_ffffffff) as *mut usize;
    let end = (&mut __bss_end as *mut usize as usize & 0x0000ffff_ffffffff) as *mut usize;
    let mut cursor = start;
    while cursor < end {
        cursor.write(0);
        cursor = cursor.offset(1);
    }
}

/// Starting from this function,
/// kernel code is running in Exception Level 1
unsafe extern fn _start_el1() -> ! {
    // Enable all co-processors
    crate::debug_boot::log("[boot: _start_el1]");
    asm!("msr cpacr_el1, $0"::"r"(0xfffffff));
    crate::debug_boot::log("[boot: zero bss]");
    zero_bss();
    // Setup paging
    crate::debug_boot::log("[boot: setup kernel pagetable]");
    crate::mm::paging::setup_kernel_pagetables();
    boot_log!("[boot: setup stack pointer]");
    SP.set(SP.get() | 0xffff0000_00000000);
    boot_log!("[boot: switch to high address space...]");
    let fn_addr = _start_el1_high_address_space as usize | 0xffff0000_00000000;
    let func: unsafe extern fn() -> ! = ::core::mem::transmute(fn_addr);
    func()
}

/// Starting from this function,
/// all kernel (virtual) addresses are located in the high address space.
/// Including SP, PC and other registers
/// i.e. `address & 0xffff0000_00000000 == 0xffff0000_00000000`
unsafe extern fn _start_el1_high_address_space() -> ! {
    crate::mm::BOOTED = true;
    println!("[boot: _start_el1_high_address_space]");
    let ptr = _start_el1_high_address_space as *const unsafe extern fn() -> !;
    crate::mm::paging::clear_temp_user_pagetable();
    // Set EL1 interrupt vector
    println!("[boot: set interrupt vector]");
    VBAR_EL1.set((&exception::exception_handlers as *const _ as usize | 0xffff0000_00000000) as _);
    barrier::isb(barrier::SY);
    // Call kmain
    set_booted();
    crate::kmain()
}
