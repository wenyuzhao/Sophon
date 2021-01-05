use core::fmt;
use core::fmt::Write;
use spin::Mutex;
use uefi::CStr16;

use crate::SYSTEM_TABLE;
// use super::IPC;



#[allow(dead_code)]
static WRITER: Mutex<Log> = Mutex::new(Log);

struct Log;

impl Write for Log {
    fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
        for c in s.chars() {
            unsafe {
                let v = [c as u16, 0];
                SYSTEM_TABLE.as_ref().unwrap().stdout().output_string(CStr16::from_u16_with_nul(&v).ok().unwrap()).unwrap();
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