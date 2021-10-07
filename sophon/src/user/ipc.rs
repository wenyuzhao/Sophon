use core::intrinsics::transmute;

use crate::task::{
    uri::{AsUri, Uri},
    *,
};

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
#[repr(usize)]
pub enum Error {
    NotFound,
    Other,
}

pub type Result<T> = core::result::Result<T, Error>;

#[repr(usize)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum IPC {
    Log = 0,
    Send,
    Receive,
    SchemeRequest,
}

#[inline]
pub fn syscall(ipc: IPC, args: &[usize]) -> isize {
    debug_assert!(args.len() <= 6);
    let a: usize = args.get(0).cloned().unwrap_or(0);
    let b: usize = args.get(1).cloned().unwrap_or(0);
    let c: usize = args.get(2).cloned().unwrap_or(0);
    let d: usize = args.get(3).cloned().unwrap_or(0);
    let e: usize = args.get(4).cloned().unwrap_or(0);
    let ret: isize;
    unsafe {
        asm!("svc #0",
            inout("x0") ipc as usize => ret,
            in("x1") a, in("x2") b, in("x3") c, in("x4") d, in("x5") e,
        );
    }
    ret
}

#[inline]
pub fn log(message: &str) {
    unsafe {
        asm!("svc #0", in("x0") IPC::Log as usize, in("x1") &message as *const &str);
    }
}

#[inline]
pub fn send(mut m: Message) {
    let ret = syscall(
        IPC::Send,
        &[unsafe { transmute::<*mut Message, _>(&mut m) }],
    );
    assert!(ret == 0, "{:?}", ret);
}

#[inline]
pub fn receive(from: Option<TaskId>) -> Message {
    unsafe {
        let mut msg: Message = ::core::mem::zeroed();
        let from_task: isize = match from {
            Some(t) => ::core::mem::transmute(t),
            None => -1,
        };
        let ret = syscall(IPC::Receive, &[transmute(from_task), transmute(&mut msg)]);
        assert!(ret == 0, "{:?}", ret);
        msg
    }
}

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
pub struct Resource(pub(crate) usize);

impl Resource {
    pub fn open(uri: impl AsUri, _flags: u32, _mode: Mode) -> Result<Resource> {
        let uri = uri.as_str();
        let fd = unsafe {
            syscall(
                IPC::SchemeRequest,
                &[
                    transmute(SchemeRequest::Open),
                    transmute(&uri),
                    transmute(&uri),
                ],
            )
        };
        Ok(Resource(fd as _))
    }

    pub fn close(self) -> Result<()> {
        unimplemented!()
    }

    pub fn stat(&self) -> Result<()> {
        unimplemented!()
    }

    pub fn lseek(&self, _offset: isize, _whence: Whence) -> Result<()> {
        unimplemented!()
    }

    pub fn read(&self, mut buf: &mut [u8]) -> Result<usize> {
        let r = unsafe {
            syscall(
                IPC::SchemeRequest,
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

    pub fn write(&self, buf: impl AsRef<[u8]>) -> Result<()> {
        let buf = buf.as_ref();
        let _ = unsafe {
            syscall(
                IPC::SchemeRequest,
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
