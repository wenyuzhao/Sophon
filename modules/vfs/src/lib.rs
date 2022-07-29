#![feature(format_args_nl)]
#![feature(default_alloc_error_handler)]
#![feature(generic_associated_types)]
#![feature(const_btree_new)]
#![no_std]

#[macro_use]
extern crate log;
extern crate alloc;

mod fs;
mod mount;
mod rootfs;

use core::mem;

use crate::fs::FileDescriptor;
use ::fs::ramfs::RamFS;
use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
};
use kernel_module::{kernel_module, KernelModule, ModuleCall, SERVICE};
use proc::ProcId;
use rootfs::ROOT_FS;
use spin::Mutex;

#[kernel_module]
pub static VFS_MODULE: VFS = VFS;

pub struct VFS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Fd(pub(crate) u32);

pub enum VFSRequest<'a> {
    Init(&'static mut RamFS),
    Open(String),
    Read(Fd, &'a mut [u8]),
}

struct ProcData {
    nodes: [Option<FileDescriptor>; 16],
    files: usize,
}

impl Default for ProcData {
    fn default() -> Self {
        let mut data = Self {
            nodes: [
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None,
            ],
            files: 0,
        };
        // data.nodes[0] = Some(FileDescriptor {
        //     node: ROOT_FS.clone(),
        //     offset: 0,
        // });
        data
    }
}

static OPEN_FILES: Mutex<BTreeMap<ProcId, ProcData>> = Mutex::new(BTreeMap::new());

impl<'a> ModuleCall for VFSRequest<'a> {
    fn from([kind, a, b, _c]: [usize; 4]) -> Self {
        match kind {
            0 => VFSRequest::Open(unsafe { (*(a as *const &str)).to_string() }),
            1 => VFSRequest::Read(Fd(a as _), unsafe { *(b as *mut &mut [u8]) }),
            usize::MAX => VFSRequest::Init(unsafe { mem::transmute(a) }),
            _ => unreachable!(),
        }
    }

    fn handle(self) -> anyhow::Result<isize> {
        match self {
            VFSRequest::Init(ramfs) => {
                ROOT_FS.init(ramfs);
                Ok(0)
            }
            VFSRequest::Open(path) => {
                let node = match fs::vfs_open(&path) {
                    Some(node) => node,
                    None => return Ok(-1),
                };
                let proc = SERVICE.current_process().unwrap();
                let mut open_files = OPEN_FILES.lock();
                let proc_data = open_files.entry(proc).or_default();
                let fd = proc_data.files;
                proc_data.nodes[fd] = Some(FileDescriptor { node, offset: 0 });
                proc_data.files += 1;
                Ok(fd as _)
            }
            VFSRequest::Read(_fd, buf) => {
                let mut open_files = OPEN_FILES.lock();
                let fd = match open_files.get_mut(&SERVICE.current_process().unwrap()) {
                    Some(proc_data) => match proc_data.nodes[_fd.0 as usize].as_mut() {
                        Some(node) => node,
                        None => return Ok(-1),
                    },
                    None => return Ok(-1),
                };
                match fd.node.fs.read(&fd.node, fd.offset, buf) {
                    None => Ok(-1),
                    Some(v) => {
                        fd.offset += v;
                        Ok(v as _)
                    }
                }
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
