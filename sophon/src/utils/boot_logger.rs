use core::{cell::UnsafeCell, fmt};
use log::Logger;
use memory::address::Address;

static BOOT_LOG: BootLogger = BootLogger(UnsafeCell::new(Address::ZERO));

#[allow(static_mut_refs)]
pub fn init(mmio: Address) {
    unsafe {
        BOOT_LOG.set_mmio_address(mmio);
        log::init(&*&raw const BOOT_LOG);
    }
}

struct BootLogger(UnsafeCell<Address>);

unsafe impl Sync for BootLogger {}

impl BootLogger {
    fn set_mmio_address(&self, addr: Address) {
        unsafe { *self.0.get() = addr }
    }
}

impl Logger for BootLogger {
    fn log(&self, s: &str) -> Result<(), fmt::Error> {
        let _guard = interrupt::uninterruptible();
        let mmio = match unsafe { *self.0.get() } {
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
