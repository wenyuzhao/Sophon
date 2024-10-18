use core::fmt::{self, Write};
use log::Log;

struct Output;

impl fmt::Write for Output {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        super::SERVICE.log(s);
        Ok(())
    }
}

struct KernelLogger;

impl Log for KernelLogger {
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
