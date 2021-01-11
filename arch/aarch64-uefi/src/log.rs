use core::fmt;
use core::fmt::Write;
use spin::Mutex;
use uefi::CStr16;
use proton_kernel::arch::*;
use crate::{BOOT_SYSTEM_TABLE, drivers::uart::{self, UART}};



#[allow(dead_code)]
static WRITER: Mutex<Log> = Mutex::new(Log);

pub struct Log;

impl Write for Log {
    fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
        if let Some(bt) = unsafe { BOOT_SYSTEM_TABLE.as_ref() } {
            for c in s.chars() {
                let v = [c as u16, 0];
                let _ = bt.stdout().output_string(CStr16::from_u16_with_nul(&v).ok().unwrap()).unwrap();
            }
        } else {
            let uart = UART.lock();
            for c in s.chars() {
                uart.putchar(c);
            }
        }
        Ok(())
    }
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    let mut writer = WRITER.lock();
    writer.write_fmt(args).unwrap();
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

impl AbstractLogger for Log {
    fn put(c: char) {
        if let Some(bt) = unsafe { BOOT_SYSTEM_TABLE.as_ref() } {
            let v = [c as u16, 0];
            let _ = bt.stdout().output_string(CStr16::from_u16_with_nul(&v).ok().unwrap()).unwrap();
        } else {
            UART.lock().putchar(c);
        }
    }
}