#![feature(format_args_nl)]
#![feature(default_alloc_error_handler)]
#![feature(generic_associated_types)]
#![no_std]

#[macro_use]
extern crate log;
extern crate alloc;

use alloc::string::{String, ToString};
use kernel_module::{kernel_module, KernelModule};

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

impl<'a> From<(usize, [usize; 3])> for VFSRequest<'a> {
    fn from((kind, args): (usize, [usize; 3])) -> Self {
        match kind {
            0 => VFSRequest::Open(unsafe { (*(args[0] as *const &str)).to_string() }),
            1 => VFSRequest::Read(Fd(args[0] as _), unsafe {
                &mut *(args[1] as *mut &mut [u8])
            }),
            _ => unreachable!(),
        }
    }
}

impl KernelModule for VFS {
    const ENABLE_MODULE_CALL: bool = true;

    fn init(&self) -> anyhow::Result<()> {
        log!("Hello, VFS!");
        Ok(())
    }

    type ModuleCall<'a> = VFSRequest<'a>;

    fn module_call(&self, call: VFSRequest) -> isize {
        match call {
            VFSRequest::Open(_) => 1,
            VFSRequest::Read(_fd, buf) => {
                let s = "hello from vfs".as_bytes();
                unsafe {
                    core::ptr::copy_nonoverlapping(s.as_ptr(), buf.as_mut_ptr(), s.len());
                }
                s.len() as isize
            }
        }
    }
}
