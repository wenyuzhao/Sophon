#![feature(format_args_nl)]
#![feature(default_alloc_error_handler)]
#![feature(generic_associated_types)]
#![no_std]

#[macro_use]
extern crate log;
extern crate alloc;

use alloc::string::{String, ToString};
use kernel_module::{kernel_module, KernelModule, ModuleCall};

#[kernel_module]
pub static VFS_MODULE: VFS = VFS;

pub struct VFS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Fd(pub(crate) u32);

pub enum VFSRequest<'a> {
    Open(String),
    Read(Fd, &'a mut [u8]),
}

impl<'a> ModuleCall for VFSRequest<'a> {
    fn from([kind, a, b, _c]: [usize; 4]) -> Self {
        match kind {
            0 => VFSRequest::Open(unsafe { (*(a as *const &str)).to_string() }),
            1 => VFSRequest::Read(Fd(a as _), unsafe { *(b as *mut &mut [u8]) }),
            _ => unreachable!(),
        }
    }

    fn handle(self) -> anyhow::Result<isize> {
        match self {
            VFSRequest::Open(_) => Ok(1),
            VFSRequest::Read(_fd, buf) => {
                let s = "hello from vfs".as_bytes();
                unsafe {
                    core::ptr::copy_nonoverlapping(s.as_ptr(), buf.as_mut_ptr(), s.len());
                }
                Ok(s.len() as isize)
            }
        }
    }
}

impl KernelModule for VFS {
    type ModuleCall<'a> = VFSRequest<'a>;

    fn init(&self) -> anyhow::Result<()> {
        log!("Hello, VFS!");
        Ok(())
    }
}
