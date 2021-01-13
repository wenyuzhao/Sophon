use core::fmt;
use core::fmt::Write;
use spin::Mutex;
use uefi::CStr16;
use proton_kernel::arch::*;
use crate::drivers::uart::{self, UART};



#[allow(dead_code)]
static WRITER: Mutex<Log> = Mutex::new(Log);

pub struct Log;

impl Write for Log {
    fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
        let uart = UART.lock();
        for c in s.chars() {
            uart.putchar(c);
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
        UART.lock().putchar(c);
    }
}