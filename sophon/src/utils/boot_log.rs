use core::fmt;
use log::Logger;
use memory::address::Address;

static mut BOOT_LOG: BootLogger = BootLogger(None);

pub fn init(mmio: Address) {
    unsafe {
        BOOT_LOG.set_mmio_address(mmio);
        log::init(&BOOT_LOG);
    }
}

struct BootLogger(Option<Address>);

impl BootLogger {
    fn set_mmio_address(&mut self, addr: Address) {
        self.0 = Some(addr);
    }
}

impl Logger for BootLogger {
    fn log(&self, s: &str) -> Result<(), fmt::Error> {
        let _guard = interrupt::uninterruptable();
        let mmio = match self.0 {
            Some(a) => a,
            _ => return Ok(()),
        };
        use crate::utils::volatile::*;
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
