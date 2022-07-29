use core::fmt;
use log::Logger;

pub struct UserLogger;

impl UserLogger {
    pub fn init() {
        log::init(&UserLogger)
    }
}

impl Logger for UserLogger {
    #[inline]
    fn log(&self, s: &str) -> Result<(), fmt::Error> {
        crate::syscall::log(s);
        Ok(())
    }
}
