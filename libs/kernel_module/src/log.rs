use crate::KernelService;
use core::fmt;
use log::Logger;

impl Logger for &dyn KernelService {
    fn log(&self, message: &str) -> Result<(), fmt::Error> {
        KernelService::log(*self, message);
        Ok(())
    }
}

pub fn init() {
    log::init(&*super::SERVICE);
}
