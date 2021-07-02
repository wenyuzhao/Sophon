use core::fmt;
use core::fmt::Write;
use proton::IPC;
use spin::Mutex;

static WRITER: Mutex<Log> = Mutex::new(Log);

struct Log;

impl Write for Log {
    fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
        IPC::log(s);
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
    ($($arg:tt)*) => ({
        $crate::log::_print(format_args_nl!($($arg)*))
    });
}
