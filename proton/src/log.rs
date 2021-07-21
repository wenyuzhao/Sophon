use crate::arch::{Arch, TargetArch};
use alloc::boxed::Box;
use core::fmt;
use core::fmt::Write;
use spin::Mutex;

#[allow(dead_code)]
pub static WRITER: Mutex<Option<Box<dyn Write + Send>>> = Mutex::new(None);

#[doc(hidden)]
#[inline(never)]
pub fn _print(args: fmt::Arguments) {
    TargetArch::uninterruptable(|| {
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
    (noeol: $($arg:tt)*) => ({
        $crate::log::_boot_print(format_args!($($arg)*))
    });
    ($($arg:tt)*) => ({
        $crate::log::_boot_print(format_args_nl!($($arg)*))
    });
}
#[doc(hidden)]
#[inline(never)]
pub fn _boot_print(args: fmt::Arguments) {
    let mut writer = BootLog;
    writer.write_fmt(args).unwrap();
}

pub struct BootLog;

impl Write for BootLog {
    fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
        use crate::utils::volatile::*;
        #[repr(C)]
        struct UARTRegisters {
            pub dr: Volatile<u32>,     // 0x00
            pub rsrecr: Volatile<u32>, // 0x04
            _0: [u8; 16],              // 0x08
            pub fr: Volatile<u32>,     // 0x18,
            _1: [u8; 4],               // 0x1c,
            pub ilpr: Volatile<u32>,   // 0x20,
            pub ibrd: Volatile<u32>,   // 0x24,
            pub fbrd: Volatile<u32>,   // 0x28,
            pub lcrh: Volatile<u32>,   // 0x2c,
            pub cr: Volatile<u32>,     // 0x30,
            pub ifls: Volatile<u32>,   // 0x34,
            pub imsc: Volatile<u32>,   // 0x38,
            pub ris: Volatile<u32>,    // 0x3c,
            pub mis: Volatile<u32>,    // 0x40,
            pub icr: Volatile<u32>,    // 0x44,
            pub dmacr: Volatile<u32>,  // 0x48,
        }
        let uart = unsafe { &mut *(crate::UART as *mut UARTRegisters) };
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
