use crate::syscall::{self, Syscall};
pub use crate::uri::*;
use core::intrinsics::transmute;

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
#[repr(usize)]
pub enum Error {
    NotFound,
    Other,
}

pub type Result<T> = core::result::Result<T, Error>;

#[repr(usize)]
pub enum SchemeRequest {
    Open = 0,
    Close,
    FStat,
    LSeek,
    Read,
    Write,
    // Stat,
}

#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Whence {
    Set,
    Cur,
    End,
}

#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    ReadOnly,
    WriteOnly,
    ReadWrite,
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Resource(pub usize);

impl Resource {
    #[inline]
    pub fn open(uri: impl AsUri, _flags: u32, _mode: Mode) -> Result<Resource> {
        let uri = uri.as_str();
        let fd = unsafe {
            syscall::syscall(
                Syscall::SchemeRequest,
                &[
                    transmute(SchemeRequest::Open),
                    transmute(&uri),
                    transmute(&uri),
                ],
            )
        };
        Ok(Resource(fd as _))
    }

    #[inline]
    pub fn close(self) -> Result<()> {
        unimplemented!()
    }

    #[inline]
    pub fn stat(&self) -> Result<()> {
        unimplemented!()
    }

    #[inline]
    pub fn lseek(&self, _offset: isize, _whence: Whence) -> Result<()> {
        unimplemented!()
    }

    #[inline]
    pub fn read(&self, mut buf: &mut [u8]) -> Result<usize> {
        let r = unsafe {
            syscall::syscall(
                Syscall::SchemeRequest,
                &[
                    transmute(SchemeRequest::Read),
                    transmute(*self),
                    transmute(&mut buf),
                ],
            )
        };
        if r < 0 {
            return Err(Error::Other);
        }
        Ok(r as _)
    }

    #[inline]
    pub fn write(&self, buf: impl AsRef<[u8]>) -> Result<()> {
        let buf = buf.as_ref();
        let _ = unsafe {
            syscall::syscall(
                Syscall::SchemeRequest,
                &[
                    transmute(SchemeRequest::Write),
                    transmute(*self),
                    transmute(&buf),
                ],
            )
        };
        Ok(())
    }
}

pub trait SchemeServer {
    fn scheme(&self) -> &'static str;
    fn open(&self, uri: &Uri, flags: u32, mode: Mode) -> Result<Resource>;
    fn close(self, fd: Resource) -> Result<()>;
    fn stat(&self, _fd: Resource) -> Result<()> {
        unimplemented!()
    }
    fn lseek(&self, _fd: Resource, _offset: isize, _whence: Whence) -> Result<()> {
        unimplemented!()
    }
    fn read(&self, fd: Resource, buf: &mut [u8]) -> Result<usize>;
    fn write(&self, fd: Resource, buf: &[u8]) -> Result<()>;
}
