pub use crate::uri::*;
use crate::{
    syscall::{self, Syscall},
    Message,
};
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
    Register = 0,
    Open,
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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct SchemeId(pub usize);

pub trait SchemeServer {
    fn name(&self) -> &str;
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

pub fn register_user_scheme(scheme: &'static impl SchemeServer) -> ! {
    let _ = unsafe {
        syscall::syscall(
            Syscall::SchemeRequest,
            &[
                transmute(SchemeRequest::Register),
                transmute(&scheme.name()),
            ],
        )
    };
    loop {
        let scheme_request = Message::receive(None);
        let args = scheme_request.get_data::<[usize; 5]>();
        let result = handle_user_scheme_request(scheme, args);
        scheme_request.reply(result);
    }
}

fn handle_user_scheme_request(scheme: &'static impl SchemeServer, args: &[usize; 5]) -> isize {
    match unsafe { transmute::<_, SchemeRequest>(args[0]) } {
        SchemeRequest::Register => -1,
        SchemeRequest::Open => {
            let uri = unsafe { transmute::<_, &&str>(args[1]) };
            let uri = Uri::new(uri).unwrap();
            let resource = scheme
                .open(&uri, args[2] as _, unsafe { transmute(args[3]) })
                .unwrap();
            unsafe { transmute(resource) }
        }
        SchemeRequest::Close => {
            unimplemented!()
        }
        SchemeRequest::FStat => {
            unimplemented!()
        }
        SchemeRequest::LSeek => {
            unimplemented!()
        }
        SchemeRequest::Read => {
            let fd = unsafe { transmute::<_, Resource>(args[1]) };
            let buf = unsafe { transmute::<_, &mut &mut [u8]>(args[2]) };
            let r = scheme.read(fd, buf).unwrap();
            unsafe { transmute(r) }
        }
        SchemeRequest::Write => {
            unimplemented!()
        }
    }
}
