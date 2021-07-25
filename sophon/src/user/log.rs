use super::ipc;
use core::fmt;
use core::fmt::Write;
use spin::Mutex;

#[allow(dead_code)]
static WRITER: Mutex<Log> = Mutex::new(Log);

struct Log;

impl Write for Log {
    fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
        ipc::log(s);
        Ok(())
    }
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    let mut writer = WRITER.lock();
    writer.write_fmt(args).unwrap();
}

#[cfg(not(feature = "kernel"))]
#[macro_export]
macro_rules! log {
    (noeol: $($arg:tt)*) => ({
        $crate::user::log::_print(format_args!($($arg)*))
    });
    ($($arg:tt)*) => ({
        $crate::user::log::_print(format_args_nl!($($arg)*))
    });
}
