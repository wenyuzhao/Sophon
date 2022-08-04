#![no_std]
#![feature(const_btree_new)]

use alloc::{
    borrow::{Cow, ToOwned},
    string::String,
    vec::Vec,
};
use proc::ProcId;
use ramfs::RamFS;
use syscall::{ModuleRequest, RawModuleRequest};

extern crate alloc;

pub mod ramfs;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Fd(pub u32);

impl Fd {
    pub const STDIN: Self = Fd(0);
    pub const STDOUT: Self = Fd(1);
    pub const STDERR: Self = Fd(2);
}

#[derive(Clone)]
pub struct Node {
    pub name: Cow<'static, str>,
    pub path: Cow<'static, str>,
    pub fs: &'static dyn FileSystem,
    pub mount: Option<usize>,
    pub block: usize,
    pub offset: usize,
}

pub struct Stat {
    pub fs: &'static dyn FileSystem,
    pub mount: Option<usize>,
    pub is_dir: bool,
}

pub trait FileSystem: Sync + Send {
    fn name(&self) -> &'static str;
    fn stat(&self, parent: &Node, file: &str) -> Option<Stat>;
    // File operations
    fn open(&self, parent: &Node, file: &str) -> Option<Node>;
    fn close(&self, node: &Node);
    fn read(&self, node: &Node, offset: usize, buf: &mut [u8]) -> Option<usize>;
    fn write(&self, node: &Node, offset: usize, buf: &[u8]) -> Option<usize>;
    // Dir operations
    fn read_dir(&self, node: &Node) -> Option<Vec<String>>;
    // Mount
    fn mount(&self, parent: &Node, file: &str, key: usize) -> Option<Node>;
}

// Possible syscalls:
// open, close, read, write, link, unlink, stat, fstat, lseek, isatty
// readdir, mkdir

pub enum VFSRequest<'a> {
    Init(&'static mut RamFS),
    Open(&'a str),
    Close(Fd),
    Read(Fd, &'a mut [u8]),
    Write(Fd, &'a [u8]),
    ReadDir(Fd, usize, &'a mut [u8]),
    Mount {
        path: &'a str,
        dev: usize,
        fs: &'a str,
    },
    RegisterFS(&'a &'static dyn FileSystem),
    ProcStart(ProcId),
    ProcExit(ProcId),
}

impl<'a> ModuleRequest<'a> for VFSRequest<'a> {
    fn as_raw(&'a self) -> RawModuleRequest<'a> {
        match self {
            Self::Init(ramfs) => RawModuleRequest::new(0, ramfs, &(), &()),
            Self::Open(s) => RawModuleRequest::new(1, s, &(), &()),
            Self::Close(fd) => RawModuleRequest::new(2, &fd.0, &(), &()),
            Self::Read(fd, buf) => RawModuleRequest::new(3, &fd.0, buf, &()),
            Self::Write(fd, buf) => RawModuleRequest::new(4, &fd.0, buf, &()),
            Self::ReadDir(fd, i, buf) => RawModuleRequest::new(5, &fd.0, i, buf),
            Self::Mount { path, dev, fs } => RawModuleRequest::new(6, path, dev, fs),
            Self::RegisterFS(ramfs) => RawModuleRequest::new(7, ramfs, &(), &()),
            Self::ProcStart(id) => RawModuleRequest::new(8, &id.0, &(), &()),
            Self::ProcExit(id) => RawModuleRequest::new(9, &id.0, &(), &()),
        }
    }
    fn from_raw(raw: RawModuleRequest<'a>) -> Self {
        match raw.id() {
            0 => Self::Init(raw.arg(0)),
            1 => Self::Open(raw.arg(0)),
            2 => Self::Close(Fd(raw.arg(0))),
            3 => Self::Read(Fd(raw.arg(0)), raw.arg(1)),
            4 => Self::Write(Fd(raw.arg(0)), raw.arg(1)),
            5 => Self::ReadDir(Fd(raw.arg(0)), raw.arg(1), raw.arg(2)),
            6 => Self::Mount {
                path: raw.arg(0),
                dev: raw.arg(1),
                fs: raw.arg(2),
            },
            7 => Self::RegisterFS(raw.arg(0)),
            8 => Self::ProcStart(ProcId(raw.arg(0))),
            9 => Self::ProcExit(ProcId(raw.arg(0))),
            _ => panic!("Unknown request"),
        }
    }
}

pub fn open(path: &str) -> Option<Fd> {
    let ret = syscall::module_call("vfs", &VFSRequest::Open(path));
    if ret < 0 {
        None
    } else {
        Some(Fd(ret as u32))
    }
}

pub fn close(fd: Fd) {
    syscall::module_call("vfs", &VFSRequest::Close(fd));
}

pub fn read(fd: Fd, buf: &mut [u8]) -> Result<usize, ()> {
    let ret = syscall::module_call("vfs", &VFSRequest::Read(fd, buf));
    if ret < 0 {
        Err(())
    } else {
        Ok(ret as usize)
    }
}

pub fn write(fd: Fd, buf: &[u8]) -> Result<usize, ()> {
    let ret = syscall::module_call("vfs", &VFSRequest::Write(fd, buf));
    if ret < 0 {
        Err(())
    } else {
        Ok(ret as usize)
    }
}

pub fn readdir(fd: Fd, i: usize) -> Result<Option<String>, ()> {
    let mut buf = [0u8; 256];
    let ret = syscall::module_call("vfs", &VFSRequest::ReadDir(fd, i, &mut buf));
    if ret < 0 {
        Err(())
    } else if ret == 0 {
        Ok(None)
    } else {
        let end = buf.iter().position(|&x| x == 0).unwrap_or(buf.len());
        Ok(core::str::from_utf8(&buf[..end]).map(|s| s.to_owned()).ok())
    }
}
