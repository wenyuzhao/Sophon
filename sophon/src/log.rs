use alloc::boxed::Box;
use core::fmt;
use core::fmt::Write;
use memory::address::Address;
use spin::Mutex;

#[allow(dead_code)]
pub static WRITER: Mutex<Option<Box<dyn Write + Send>>> = Mutex::new(None);

#[doc(hidden)]
#[inline(never)]
pub fn _print(args: fmt::Arguments) {
    interrupt::uninterruptable(|| {
        let mut writer = WRITER.lock();
        if let Some(writer) = writer.as_mut() {
            writer.write_fmt(args).unwrap();
        }
    });
}

#[macro_export]
macro_rules! log {
    (noeol: $($arg:tt)*) => ({
        $crate::log::_print(format_args!($($arg)*))
    });
    ($($arg:tt)*) => ({
        $crate::log::_print(format_args_nl!($($arg)*))
    });
}
#[macro_export]
macro_rules! boot_log {
    ($($arg:tt)*) => ({
        $crate::log::_boot_print(format_args_nl!($($arg)*))
    });
}

pub static mut BOOT_LOG: BootLog = BootLog(None);

#[doc(hidden)]
#[inline(never)]
pub fn _boot_print(args: fmt::Arguments) {
    unsafe {
        BOOT_LOG.write_fmt(args).unwrap();
    }
}

pub struct BootLog(Option<Address>);

impl BootLog {
    pub fn set_mmio_address(&mut self, addr: Address) {
        self.0 = Some(addr);
    }
}

impl Write for BootLog {
    fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
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
