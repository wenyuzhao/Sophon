#![no_std]

use syscall::{ModuleRequest, RawModuleRequest};

extern crate alloc;

pub trait Device: Send + Sync {
    fn name(&self) -> &'static str;
    fn read(&self, offset: usize, buf: &mut [u8]) -> Option<usize>;
    fn write(&self, offset: usize, buf: &[u8]) -> Option<usize>;
}

pub enum DevRequest<'a> {
    RegisterDev(&'a &'static dyn Device),
}

impl<'a> ModuleRequest<'a> for DevRequest<'a> {
    fn as_raw(&'a self) -> RawModuleRequest<'a> {
        match self {
            Self::RegisterDev(dev) => RawModuleRequest::new(0, dev, &(), &()),
        }
    }
    fn from_raw(raw: RawModuleRequest<'a>) -> Self {
        match raw.id() {
            0 => Self::RegisterDev(raw.arg(0)),
            _ => panic!("Unknown request"),
        }
    }
}
