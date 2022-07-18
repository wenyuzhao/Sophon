#![feature(format_args_nl)]
#![feature(default_alloc_error_handler)]
#![no_std]

#[macro_use]
extern crate log;

use kernel_module::{kernel_module, KernelModule};

#[kernel_module]
pub static VFS_MODULE: VFS = VFS;

pub struct VFS;

impl KernelModule for VFS {
    const ENABLE_MODULE_CALL: bool = true;

    fn init(&self) -> anyhow::Result<()> {
        log!("Hello, VFS!");
        Ok(())
    }

    fn module_call(&self, kind: usize, args: [usize; 3]) -> isize {
        match kind {
            0 => 1,
            1 => {
                let buf = unsafe { &mut *(args[1] as *mut &mut [u8]) };
                let s = "hello from vfs".as_bytes();
                for x in 0..s.len() {
                    (*buf)[x] = s[x];
                }
                s.len() as isize
            }
            _ => unreachable!(),
        }
    }
}
