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

impl Uri<'_> {
    #[inline]
    pub fn open(uri: impl AsUri) -> Result<Resource> {
        let uri = uri.as_str();
        let mut resource: Resource = Resource(0);
        send(Message::new(TaskId::NULL, TaskId::KERNEL).with_data((0usize, &uri, &mut resource)));
        Ok(resource)
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Resource(pub(crate) usize);

impl Resource {
    pub fn read(&self, mut buf: &mut [u8]) -> Result<()> {
        send(Message::new(TaskId::NULL, TaskId::KERNEL).with_data((1usize, *self, &mut buf)));
        Ok(())
    }

    pub fn write(&self, _buf: &[u8]) -> Result<()> {
        unimplemented!()
    }
}

pub trait SchemeServer {
    fn scheme(&self) -> &'static str;
    fn register(&self) -> ! {
        loop {
            let m = Message::receive(None);
            let args = m.get_data::<[u64; 6]>();
            match args[0] {
                0 => {
                    let uri = unsafe { transmute::<_, &Uri>(args[1]) };
                    let resource = self.open(uri).unwrap();
                    m.reply(resource);
                }
                1 => {
                    let fd = unsafe { transmute::<_, Resource>(args[1]) };
                    let buf = unsafe { transmute::<_, &mut &mut [u8]>(args[2]) };
                    self.read(fd, buf).unwrap();
                    m.reply(0usize);
                }
                2 => {
                    let fd = unsafe { transmute::<_, Resource>(args[1]) };
                    let buf = unsafe { transmute::<_, &&[u8]>(args[2]) };
                    self.write(fd, buf).unwrap();
                    m.reply(0usize);
                }
                _ => unimplemented!(),
            }
        }
    }
    fn open(&self, uri: &Uri) -> Result<Resource>;
    fn read(&self, fd: Resource, buf: &mut [u8]) -> Result<()>;
    fn write(&self, fd: Resource, buf: &[u8]) -> Result<()>;
}
