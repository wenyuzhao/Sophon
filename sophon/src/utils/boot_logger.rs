use core::fmt::{self};
use memory::address::Address;

struct BootOutput(Address);

impl fmt::Write for BootOutput {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let _guard = interrupt::uninterruptible();
        let mmio = match self.0 {
            a if !a.is_zero() => a,
            _ => return Ok(()),
        };
        use memory::volatile::*;
        #[repr(C)]
        struct UARTRegisters {
            pub dr: Volatile<u32>,     // 0x00
            pub rsrecr: Volatile<u32>, // 0x04
            _0: [u8; 16],              // 0x08
            pub fr: Volatile<u32>,     // 0x18,
        }
        let uart = unsafe { mmio.as_mut::<UARTRegisters>() };
        let mut putc = |c: char| {
            while uart.fr.get() & (1 << 5) != 0 {}
            uart.dr.set(c as u8 as u32);
        };
        for c in s.chars() {
            if c == '\n' {
                putc('\r')
            }
            putc(c)
        }
        Ok(())
    }
}

static mut BOOT_OUTPUT: BootOutput = BootOutput(Address::ZERO);

#[allow(static_mut_refs)]
pub fn init(mmio: Address) {
    unsafe {
        BOOT_OUTPUT.0 = mmio;
        super::print::init(&mut BOOT_OUTPUT);
    }
}
