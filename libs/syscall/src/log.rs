use core::fmt;
use core::fmt::Write;
use log::Log;

struct Stdout;

impl fmt::Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        crate::syscall::log(s);
        Ok(())
    }
}

pub struct UserLogger;

static LOGGER: UserLogger = UserLogger;

impl UserLogger {
    pub fn init() {
        log::set_logger(&LOGGER).unwrap();
    }
}

impl Log for UserLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    #[inline]
    fn log(&self, record: &log::Record) {
        let mut stdout = Stdout;
        writeln!(stdout, "[{}] {}", record.level(), record.args()).unwrap();
    }

    fn flush(&self) {}
}
