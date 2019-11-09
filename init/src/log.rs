use core::fmt;
use core::fmt::Write;
use core::ptr;
use spin::Mutex;
use crate::syscall::SysCall;

static WRITER: Mutex<Log> = Mutex::new(Log);

struct Log;

impl Write for Log {
    fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
        syscall!(SysCall::Log, &s);
        Ok(())
    }
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    let mut writer = WRITER.lock();
    writer.write_fmt(args);
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => ({
        $crate::log::_print(format_args_nl!($($arg)*))
    });
}