#![feature(format_args_nl)]
#![feature(default_alloc_error_handler)]
#![feature(generic_associated_types)]
#![feature(const_btree_new)]
#![feature(box_syntax)]
#![no_std]

#[macro_use]
extern crate log;
extern crate alloc;
mod fs;
mod mount;
mod rootfs;

use crate::fs::FileDescriptor;
use alloc::{borrow::ToOwned, boxed::Box, collections::BTreeMap, string::String};
use kernel_module::{kernel_module, KernelModule, SERVICE};
use proc::ProcId;
use rootfs::ROOT_FS;
use spin::{Mutex, RwLock};
use vfs::{FileSystem, VFSRequest};

#[kernel_module]
pub static VFS_MODULE: VFS = VFS;

pub struct VFS;

struct ProcData {
    nodes: [Option<FileDescriptor>; 16],
    files: usize,
}

impl ProcData {
    fn new() -> Box<Self> {
        let data = box Self {
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

static OPEN_FILES: Mutex<BTreeMap<ProcId, Box<ProcData>>> = Mutex::new(BTreeMap::new());

impl KernelModule for VFS {
    type ModuleRequest<'a> = VFSRequest<'a>;

    fn init(&self) -> anyhow::Result<()> {
        log!("Hello, VFS!");
        Ok(())
    }
    fn handle_module_call<'a>(&self, privileged: bool, request: Self::ModuleRequest<'a>) -> isize {
        match request {
            VFSRequest::Init(ramfs) => {
                assert!(privileged);
                ROOT_FS.init(ramfs);
                0
            }
            VFSRequest::Open(path) => {
                assert!(!privileged);
                let node = match fs::vfs_open(&path) {
                    Some(node) => node,
                    None => return -1,
                };
                let proc = SERVICE.current_process().unwrap();
                let mut open_files = OPEN_FILES.lock();
                let proc_data = open_files.entry(proc).or_insert_with(|| ProcData::new());
                let fd = proc_data.files;
                proc_data.nodes[fd] = Some(FileDescriptor { node, offset: 0 });
                proc_data.files += 1;
                fd as _
            }
            VFSRequest::Read(_fd, buf) => {
                assert!(!privileged);
                let mut open_files = OPEN_FILES.lock();
                let fd = match open_files.get_mut(&SERVICE.current_process().unwrap()) {
                    Some(proc_data) => match proc_data.nodes[_fd.0 as usize].as_mut() {
                        Some(node) => node,
                        None => return -1,
                    },
                    None => return -1,
                };
                match fd.node.fs.read(&fd.node, fd.offset, buf) {
                    None => -1,
                    Some(v) => {
                        fd.offset += v;
                        v as _
                    }
                }
            }
            VFSRequest::Mount { path, dev, fs } => {
                assert!(privileged);
                let fs = FILE_SYSTEMS.read()[fs];
                mount::vfs_mount(&path, dev, unsafe { &*(fs as *const dyn FileSystem) }).unwrap();
                0
            }
            VFSRequest::RegisterFS(fs) => {
                log!("RegisterFS");

                crate::FILE_SYSTEMS
                    .write()
                    .insert(fs.name().to_owned(), fs.to_owned());
                0
            }
        }
    }
}

static FILE_SYSTEMS: RwLock<BTreeMap<String, &'static dyn FileSystem>> =
    RwLock::new(BTreeMap::new());
