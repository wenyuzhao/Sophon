use core::fmt::{self, Write};

use crate::SERVICE;

struct Output;

impl fmt::Write for Output {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        super::SERVICE.log(s);
        Ok(())
    }
}

struct KernelLogger;

impl log::Log for KernelLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        let mut stdout = Output;
        writeln!(stdout, "[{}] {}", record.level(), record.args()).unwrap();
    }

    fn flush(&self) {}
}

static LOGGER: KernelLogger = KernelLogger;

pub fn init() {
    log::set_logger(&LOGGER).unwrap();
}

#[doc(hidden)]
#[inline(never)]
#[allow(static_mut_refs)]
pub fn _log(args: core::fmt::Arguments) {
    let mut log = Log;
    log.write_fmt(args).unwrap();
    log.write_char('\n').unwrap();
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => ({
        $crate::log::_log(format_args_nl!($($arg)*))
    });
}

struct Log;

impl Write for Log {
    fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
        SERVICE.log(s);
        Ok(())
    }
}
