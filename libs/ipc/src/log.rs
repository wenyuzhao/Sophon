use core::fmt::{self, Write};
use spin::Mutex;

#[allow(dead_code)]
static WRITER: Mutex<Log> = Mutex::new(Log);

struct Log;

impl Write for Log {
    #[inline]
    fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
        crate::syscall::log(s);
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
