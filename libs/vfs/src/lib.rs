#![no_std]
#![feature(const_btree_new)]

use alloc::{borrow::Cow, string::String, vec::Vec};
use ramfs::RamFS;
use syscall::{ModuleRequest, RawModuleRequest};

extern crate alloc;

pub mod ramfs;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Fd(pub u32);

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
    fn read(&self, node: &Node, offset: usize, buf: &mut [u8]) -> Option<usize>;
    // Dir operations
    fn read_dir(&self, node: &Node) -> Option<Vec<String>>;
    // Mount
    fn mount(&self, parent: &Node, file: &str, key: usize) -> Option<Node>;
}

pub enum VFSRequest<'a> {
    Init(&'static mut RamFS),
    Open(&'a str),
    Read(Fd, &'a mut [u8]),
    Mount {
        path: &'a str,
        dev: usize,
        fs: &'a str,
    },
    RegisterFS(&'a &'static dyn FileSystem),
}

impl<'a> ModuleRequest<'a> for VFSRequest<'a> {
    fn as_raw(&'a self) -> RawModuleRequest<'a> {
        match self {
            Self::Init(ramfs) => RawModuleRequest::new(0, ramfs, &(), &()),
            Self::Open(s) => RawModuleRequest::new(1, s, &(), &()),
            Self::Read(fd, buf) => RawModuleRequest::new(2, &fd.0, buf, &()),
            Self::Mount { path, dev, fs } => RawModuleRequest::new(3, path, dev, fs),
            Self::RegisterFS(ramfs) => RawModuleRequest::new(4, ramfs, &(), &()),
        }
    }
    fn from_raw(raw: RawModuleRequest<'a>) -> Self {
        match raw.id() {
            0 => Self::Init(raw.arg(0)),
            1 => Self::Open(raw.arg(0)),
            2 => Self::Read(Fd(raw.arg(0)), raw.arg(1)),
            3 => Self::Mount {
                path: raw.arg(0),
                dev: raw.arg(1),
                fs: raw.arg(2),
            },
            4 => Self::RegisterFS(raw.arg(0)),
            _ => panic!("Unknown request"),
        }
    }
}

pub fn open(path: &str) -> isize {
    syscall::module_call("vfs", &VFSRequest::Open(path))
}

pub fn read(fd: usize, buf: &mut [u8]) -> isize {
    syscall::module_call("vfs", &VFSRequest::Read(Fd(fd as _), buf))
}
