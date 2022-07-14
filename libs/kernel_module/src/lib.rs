#![no_std]

use core::{fmt, ops::Deref};

use log::Logger;

pub trait KernelService: Send + Sync + 'static {
    fn log(&self, s: &str);
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct KernelServiceWrapper([usize; 2]);

impl KernelServiceWrapper {
    pub fn get_service(self) -> &'static dyn KernelService {
        unsafe { core::mem::transmute(self) }
    }
    pub fn from_service(service: &'static dyn KernelService) -> Self {
        unsafe { core::mem::transmute(service) }
    }
}

impl Deref for KernelServiceWrapper {
    type Target = dyn KernelService;

    fn deref(&self) -> &'static Self::Target {
        self.get_service()
    }
}

impl Logger for &dyn KernelService {
    fn log(&self, message: &str) -> Result<(), fmt::Error> {
        KernelService::log(*self, message);
        Ok(())
    }
}

static mut SERVICE: Option<&'static dyn KernelService> = None;

pub fn init(service: KernelServiceWrapper) {
    unsafe {
        SERVICE = Some(service.get_service());
        log::init(SERVICE.as_ref().unwrap());
    }
}
