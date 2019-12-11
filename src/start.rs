use crate::exception;
use crate::mm::paging;
use cortex_a::{asm, regs::*, barrier};

/// Kernel entry code, loaded at `0x80000`
/// 
/// Shoud running in Exception Level 2
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
    asm!("mov sp, $0"::"r"(0x80000));
    
    
    // loop {}
    
    crate::debug_boot::UART::init();
    // boot_log!("xxx");
    // Switch from EL2 -> EL1
    // if CurrentEL.get() & 3 == 0 {
        // crate::debug_boot::log("[boot el3]");    
    // }
    assert!(CurrentEL.get() == CurrentEL::EL::EL2.value);
    crate::debug_boot::log("[boot..x.]");
    
    // loop {}
    // boot_log!("yyy");
    CNTHCTL_EL2.write(CNTHCTL_EL2::EL1PCEN::SET + CNTHCTL_EL2::EL1PCTEN::SET);
    // No offset for reading the counters.
    CNTVOFF_EL2.set(0);
    // Disable MMU
    SCTLR_EL1.set((3 << 28) | (3 << 22) | (1 << 20) | (1 << 11));
    // loop {}
    // Set execution mode = AArch64
    // HCR_EL2.set(HCR_EL2.get() | (1 << 32));
    HCR_EL2.write(HCR_EL2::RW::EL1IsAarch64);

    // boot_log!("zzz");
    // Enable Debug+SError+IRQ+FIQ+EL1h
    SPSR_EL2.write(SPSR_EL2::D::Masked + SPSR_EL2::A::Masked + SPSR_EL2::I::Masked + SPSR_EL2::F::Masked + SPSR_EL2::M::EL1h);
    // SPSR_EL1.write(SPSR_EL1::D::Masked + SPSR_EL1::A::Masked + SPSR_EL1::I::Masked + SPSR_EL1::F::Masked);
    // boot_log!("[boot... 1]");
    // boot_log!("qqq");
    // loop {}
    // Switch to EL1 -> kmain
    // boot_log!("ELR_EL2 {:?}", crate::kmain as *const ());
    
    // boot_log!("[boot: 0]");
    // loop {}
    // loop {}
    ELR_EL2.set(_start_el1 as *const () as u64); // EL1 PC after return from `eret`
    // boot_log!("[boot: 1]");
    // SP_EL0.set(0x80000); // EL1 stack
    SP_EL1.set(0x80000); // EL1 stack
    // boot_log!("[boot... 2]");
    // loop {}
    // crate::kmain();
    // boot_log!("[boot: eret]");
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
/// 
/// kernel code is running in Exception Level 1
unsafe extern fn _start_el1() -> ! {
    // Enable all co-processors
    crate::debug_boot::log("[boot: _start_el1]");
    asm!("msr cpacr_el1, $0"::"r"(0xfffffff));
    zero_bss();
    crate::debug_boot::log("[boot: bss zeroed]");
    // SPSR_EL1.write(SPSR_EL1::D::Masked + SPSR_EL1::A::Masked + SPSR_EL1::I::Masked + SPSR_EL1::F::Masked + SPSR_EL1::M::EL0t);
    // crate::debug_boot::log("[boot: cpacr_el1 is set]");
    // Setup paging
    crate::debug_boot::log("setup_kernel_pagetables");
    
    crate::mm::paging::setup_kernel_pagetables();
    // loop {}
    boot_log!("[boot: setup_kernel_pagetables finished]");
    SP.set(SP.get() | 0xffff0000_00000000);
    boot_log!("[boot: sp set {:x}]", SP.get());
    // Call _start_el1_upper_address
    let fn_addr = _start_el1_high_address_space as usize | 0xffff0000_00000000;
    let func: unsafe extern fn() -> ! = ::core::mem::transmute(fn_addr);
    // println!("{:?}", func as *const unsafe extern fn() -> !);
    func()
}

/// Starting from this function,
/// 
/// all kernel (virtual) addresses are located in the high address space.
/// 
/// Including SP, PC and other registers
/// 
/// i.e. `address & 0xffff0000_00000000 == 0xffff0000_00000000`
unsafe extern fn _start_el1_high_address_space() -> ! {
    crate::mm::BOOTED = true;
    // boot_log!("[boot: _start_el1_high_address_space]");
    println!("[boot: _start_el1_high_address_space]");
    let ptr = _start_el1_high_address_space as *const unsafe extern fn() -> !;
    // println!("{:?}", ptr);
    crate::mm::paging::clear_temp_user_pagetable();
    println!("{:?}", ptr);
    // println!("[boot: _start_el1_high_address_space 2]");
    // assert!(SP.get() & 0xffff0000_00000000 == 0xffff0000_00000000);
    // Set EL1 interrupt vector
    VBAR_EL1.set((&exception::exception_handlers as *const _ as usize | 0xffff0000_00000000) as _);
    barrier::isb(barrier::SY);
    // Call kmain
    crate::kmain()
}
