#![feature(format_args_nl)]
#![feature(downcast_unchecked)]
#![no_std]

#[macro_use]
extern crate kernel_module;
extern crate alloc;
mod fs;
mod mount;
mod rootfs;

use core::any::Any;

use crate::fs::FileDescriptor;
use alloc::{
    borrow::ToOwned, boxed::Box, collections::BTreeMap, format, string::String, vec, vec::Vec,
};
use kernel_module::{kernel_module, KernelModule, SERVICE};
use proc::{Proc, ProcId};
use rootfs::ROOT_FS;
use spin::{Mutex, RwLock};
use vfs::{ramfs::RamFS, FileSystem, VFSManager, VFSRequest};

#[kernel_module]
pub static VFS: VFS = VFS {};

pub struct VFS {}

impl VFS {
    #[inline]
    fn get_current_state(&self) -> Option<&Mutex<ProcData>> {
        Some(self.get_state(&*SERVICE.process_manager().current_proc()?))
    }

    #[inline]
    fn get_state(&self, proc: &dyn Proc) -> &Mutex<ProcData> {
        let state = proc.fs() as *const dyn Any;
        unsafe { (*state).downcast_ref_unchecked::<Mutex<ProcData>>() }
    }
}

impl VFSManager for VFS {
    fn init(&self, ramfs: &'static mut RamFS) {
        ROOT_FS.init(ramfs);
    }

    fn register_process(&self, _proc: ProcId, cwd: String) -> Box<dyn Any> {
        Box::new(Mutex::new(ProcData::new(cwd)))
    }

    fn deregister_process(&self, _proc: ProcId) {}

    fn register_fs(&self, fs: &'static dyn FileSystem) {
        crate::FILE_SYSTEMS.write().insert(fs.name().to_owned(), fs);
    }
}

struct ProcData {
    nodes: [Option<FileDescriptor>; 16],
    cwd: String,
    files: usize,
}

impl ProcData {
    fn new(cwd: String) -> Self {
        let cwd = if cwd == "" {
            VFS.get_current_state()
                .map(|s| s.lock().cwd.clone())
                .unwrap_or_else(|| "/".to_owned())
        } else {
            cwd
        };
        let mut data = Self {
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

impl KernelModule for VFS {
    type ModuleRequest<'a> = VFSRequest<'a>;

    fn init(&'static mut self) -> anyhow::Result<()> {
        SERVICE.set_vfs_manager(self);
        Ok(())
    }

    fn handle_module_call<'a>(&self, privileged: bool, request: Self::ModuleRequest<'a>) -> isize {
        debug_assert!(!interrupt::is_enabled());
        match request {
            VFSRequest::Open(path) => {
                let mut proc_data = self.get_current_state().unwrap().lock();
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
                if fd.0 < 3 {
                    return -1;
                }
                let mut proc_data = self.get_current_state().unwrap().lock();
                let node = match proc_data.nodes[fd.0 as usize].take() {
                    Some(fd) => {
                        proc_data.files -= 1;
                        fd.node
                    }
                    None => return -1,
                };
                node.fs.close(&node);
                0
            }
            VFSRequest::Read(fd, buf) => {
                let mut proc_data = self.get_current_state().unwrap().lock();
                let fdesc = match proc_data.nodes[fd.0 as usize].as_mut() {
                    Some(fd) => fd,
                    None => return -1,
                };
                let fs = fdesc.node.fs;
                let node = fdesc.node.clone();
                let offset = fdesc.offset;
                drop(proc_data);
                match fs.read(&node, offset, buf) {
                    None => -1,
                    Some(v) => {
                        let mut proc_data = self.get_current_state().unwrap().lock();
                        let fdesc = match proc_data.nodes[fd.0 as usize].as_mut() {
                            Some(fd) => fd,
                            None => return -1,
                        };
                        fdesc.offset += v;
                        v as _
                    }
                }
            }
            VFSRequest::Write(fd, buf) => {
                let mut proc_data = self.get_current_state().unwrap().lock();
                let fdesc = match proc_data.nodes[fd.0 as usize].as_mut() {
                    Some(fd) => fd,
                    None => return -1,
                };
                let fs = fdesc.node.fs;
                let node = fdesc.node.clone();
                let offset = fdesc.offset;
                drop(proc_data);
                match fs.write(&node, offset, buf) {
                    None => -1,
                    Some(v) => {
                        let mut proc_data = self.get_current_state().unwrap().lock();
                        let fdesc = match proc_data.nodes[fd.0 as usize].as_mut() {
                            Some(fd) => fd,
                            None => return -1,
                        };
                        fdesc.offset += v;
                        v as _
                    }
                }
            }
            VFSRequest::ReadDir(fd, i, buf) => {
                let mut proc_data = self.get_current_state().unwrap().lock();
                let fdesc = match proc_data.nodes[fd.0 as usize].as_mut() {
                    Some(fd) => fd,
                    None => return -1,
                };
                if let Some(entries) = fdesc.node.fs.read_dir(&fdesc.node) {
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
            VFSRequest::GetCwd(buf) => {
                let proc_data = self.get_current_state().unwrap().lock();
                let cwd = proc_data.cwd.as_str();
                if cwd.len() > buf.len() {
                    return -1;
                }
                unsafe { core::ptr::copy_nonoverlapping(cwd.as_ptr(), buf.as_mut_ptr(), cwd.len()) }
                cwd.len() as _
            }
            VFSRequest::SetCwd(path) => {
                let mut proc_data = self.get_current_state().unwrap().lock();
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

#[test]
fn read_text_file() {
    let file = vfs::open("/etc/hello.txt").unwrap();
    let mut buf = [0u8; 32];
    let len = vfs::read(file, &mut buf).unwrap();
    let s = core::str::from_utf8(&buf[0..len]);
    assert_eq!(s, Ok("Hello world from file!"));
    vfs::close(file);
}
