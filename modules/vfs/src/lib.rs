#![feature(format_args_nl)]
#![feature(default_alloc_error_handler)]
#![feature(generic_associated_types)]
#![feature(const_btree_new)]
#![feature(box_syntax)]
#![no_std]

#[macro_use]
extern crate kernel_module;
extern crate alloc;
mod fs;
mod mount;
mod rootfs;

use crate::fs::FileDescriptor;
use alloc::{
    borrow::ToOwned, boxed::Box, collections::BTreeMap, format, string::String, vec, vec::Vec,
};
use kernel_module::{kernel_module, KernelModule, SERVICE};
use proc::ProcId;
use rootfs::ROOT_FS;
use spin::{Mutex, RwLock};
use vfs::{FileSystem, VFSRequest};

#[kernel_module]
pub static VFS: VFS = VFS {};

pub struct VFS {}

struct ProcData {
    nodes: [Option<FileDescriptor>; 16],
    cwd: String,
    files: usize,
}

impl ProcData {
    fn new(cwd: String) -> Box<Self> {
        let mut data = box Self {
            nodes: [
                None, None, None, None, None, None, None, None, None, None, None, None, None, None,
                None, None,
            ],
            cwd,
            files: 3,
        };
        let stdio = fs::vfs_open("/dev/tty.serial").unwrap();
        data.nodes[0] = Some(FileDescriptor {
            node: stdio.clone(),
            offset: 0,
        });
        data.nodes[1] = Some(FileDescriptor {
            node: stdio.clone(),
            offset: 0,
        });
        data.nodes[2] = Some(FileDescriptor {
            node: stdio.clone(),
            offset: 0,
        });
        data
    }

    fn set_cwd(&mut self, cwd: &str) -> Result<(), ()> {
        let cwd = self.canonicalize(cwd.to_owned())?;
        if !fs::dir_or_mnt_exists(&cwd) {
            return Err(());
        }
        self.cwd = cwd;
        Ok(())
    }

    fn canonicalize(&self, s: String) -> Result<String, ()> {
        if s.starts_with("/") {
            return Ok(s);
        }
        let s = s.strip_suffix("/").unwrap_or_else(|| s.as_str());
        let mut buf = if s.starts_with("/") {
            vec![]
        } else {
            self.cwd
                .strip_prefix("/")
                .unwrap()
                .split("/")
                .filter(|x| !x.is_empty())
                .collect::<Vec<_>>()
        };
        for seg in s.split("/").filter(|x| !x.is_empty()) {
            if seg == "." || seg == "" {
                continue;
            } else if seg == ".." {
                if !buf.is_empty() {
                    buf.pop();
                } else {
                    return Err(());
                }
            } else {
                buf.push(seg);
            }
        }
        Ok(format!("/{}", buf.join("/")))
    }
}

static OPEN_FILES: Mutex<BTreeMap<ProcId, Box<ProcData>>> = Mutex::new(BTreeMap::new());

impl KernelModule for VFS {
    type ModuleRequest<'a> = VFSRequest<'a>;

    fn init(&mut self) -> anyhow::Result<()> {
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
                let proc = SERVICE.current_process().unwrap();
                let mut open_files = OPEN_FILES.lock();
                let proc_data = open_files.get_mut(&proc).unwrap();
                let path = match proc_data.canonicalize(path.to_owned()) {
                    Ok(path) => path,
                    Err(_) => return -1,
                };
                let node = match fs::vfs_open(&path) {
                    Some(node) => node,
                    None => return -1,
                };
                let node = if let Some(mnt) = node.mount {
                    let mnt_table = mount::MOUNT_POINTS.read();
                    let mnt = mnt_table[mnt].as_ref().unwrap();
                    mnt.root.clone()
                } else {
                    node
                };
                let fd = proc_data.files;
                proc_data.nodes[fd] = Some(FileDescriptor { node, offset: 0 });
                proc_data.files += 1;
                fd as _
            }
            VFSRequest::Close(fd) => {
                assert!(!privileged);
                if fd.0 < 3 {
                    return -1;
                }
                let mut open_files = OPEN_FILES.lock();
                let node = match open_files.get_mut(&SERVICE.current_process().unwrap()) {
                    Some(proc_data) => match proc_data.nodes[fd.0 as usize].take() {
                        Some(fd) => {
                            proc_data.files -= 1;
                            fd.node
                        }
                        None => return -1,
                    },
                    None => return -1,
                };
                node.fs.close(&node);
                0
            }
            VFSRequest::Read(fd, buf) => {
                assert!(!privileged);
                let mut open_files = OPEN_FILES.lock();
                let fd = match open_files.get_mut(&SERVICE.current_process().unwrap()) {
                    Some(proc_data) => match proc_data.nodes[fd.0 as usize].as_mut() {
                        Some(fd) => fd,
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
            VFSRequest::Write(fd, buf) => {
                assert!(!privileged);
                let mut open_files = OPEN_FILES.lock();
                let fd = match open_files.get_mut(&SERVICE.current_process().unwrap()) {
                    Some(proc_data) => match proc_data.nodes[fd.0 as usize].as_mut() {
                        Some(fd) => fd,
                        None => return -1,
                    },
                    None => return -1,
                };
                match fd.node.fs.write(&fd.node, fd.offset, buf) {
                    None => -1,
                    Some(v) => {
                        fd.offset += v;
                        v as _
                    }
                }
            }
            VFSRequest::ReadDir(fd, i, buf) => {
                let mut open_files = OPEN_FILES.lock();
                let fd = match open_files.get_mut(&SERVICE.current_process().unwrap()) {
                    Some(proc_data) => match proc_data.nodes[fd.0 as usize].as_mut() {
                        Some(fd) => fd,
                        None => return -1,
                    },
                    None => return -1,
                };
                if let Some(entries) = fd.node.fs.read_dir(&fd.node) {
                    if i >= entries.len() {
                        0
                    } else {
                        let s = entries[i].as_bytes();
                        let len = usize::min(s.len(), buf.len());
                        unsafe { core::ptr::copy_nonoverlapping(s.as_ptr(), buf.as_mut_ptr(), len) }
                        1
                    }
                } else {
                    -1
                }
            }
            VFSRequest::Mount { path, dev, fs } => {
                assert!(privileged);
                let fs = FILE_SYSTEMS.read()[fs];
                mount::vfs_mount(&path, dev, unsafe { &*(fs as *const dyn FileSystem) }).unwrap();
                0
            }
            VFSRequest::RegisterFS(fs) => {
                crate::FILE_SYSTEMS
                    .write()
                    .insert(fs.name().to_owned(), fs.to_owned());
                0
            }
            VFSRequest::ProcStart { proc, parent, cwd } => {
                assert!(privileged);
                let mut open_files = OPEN_FILES.lock();
                let cwd = if cwd == "" || !fs::dir_or_mnt_exists(cwd) {
                    open_files
                        .get(&parent)
                        .map(|x| x.cwd.clone())
                        .unwrap_or_else(|| "/".to_owned())
                } else {
                    cwd.to_owned()
                };
                open_files.insert(proc, ProcData::new(cwd.to_owned()));
                0
            }
            VFSRequest::ProcExit(proc_id) => {
                assert!(privileged);
                let mut open_files = OPEN_FILES.lock();
                open_files.remove(&proc_id);
                0
            }
            VFSRequest::GetCwd(buf) => {
                let mut open_files = OPEN_FILES.lock();
                let cwd = match open_files
                    .get_mut(&SERVICE.current_process().unwrap())
                    .map(|proc_data| proc_data.cwd.as_str())
                {
                    Some(x) => x,
                    _ => return -1,
                };
                if cwd.len() > buf.len() {
                    return -1;
                }
                unsafe { core::ptr::copy_nonoverlapping(cwd.as_ptr(), buf.as_mut_ptr(), cwd.len()) }
                cwd.len() as _
            }
            VFSRequest::SetCwd(path) => {
                let mut open_files = OPEN_FILES.lock();
                let proc_data = match open_files.get_mut(&SERVICE.current_process().unwrap()) {
                    Some(proc_data) => proc_data,
                    None => return -1,
                };
                match proc_data.set_cwd(path) {
                    Ok(_) => 0,
                    Err(_) => -1,
                }
            }
        }
    }
}

static FILE_SYSTEMS: RwLock<BTreeMap<String, &'static dyn FileSystem>> =
    RwLock::new(BTreeMap::new());
