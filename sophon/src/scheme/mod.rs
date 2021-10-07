use core::intrinsics::transmute;

use alloc::{boxed::Box, collections::BTreeMap};
use spin::Mutex;

use crate::{
    task::{uri::Uri, Task},
    user::ipc::{Mode, Resource, Result as IoResult, SchemeRequest, SchemeServer},
};

pub static SCHEMES: Mutex<BTreeMap<&'static str, Box<dyn SchemeServer + Send>>> =
    Mutex::new(BTreeMap::new());

struct SystemSchemeServer {}

impl SchemeServer for SystemSchemeServer {
    fn scheme(&self) -> &'static str {
        "system"
    }
    fn open(&self, _uri: &Uri, _flags: u32, _mode: Mode) -> IoResult<Resource> {
        log!("SystemSchemeServer 0");
        Ok(Resource(0))
    }
    fn close(self, _fd: Resource) -> IoResult<()> {
        unimplemented!()
    }
    fn read(&self, _fd: Resource, buf: &mut [u8]) -> IoResult<usize> {
        let msg = "test\0".as_bytes();
        let mut cursor = 0;
        for b in msg {
            if cursor >= buf.len() {
                break;
            }
            buf[cursor] = *b;
            cursor += 1;
        }
        Ok(cursor)
    }
    fn write(&self, _fd: Resource, buf: &[u8]) -> IoResult<()> {
        let s = core::str::from_utf8(buf).unwrap();
        log!("{}", s);
        Ok(())
    }
}

pub fn register_kernel_schemes() {
    SCHEMES.lock().insert("system", box SystemSchemeServer {});
}

pub fn handle_scheme_request(args: &[usize; 5]) -> Result<isize, isize> {
    match unsafe { transmute::<_, SchemeRequest>(args[0]) } {
        SchemeRequest::Open => {
            let uri = unsafe { transmute::<_, &&str>(args[1]) };
            let uri = Uri::new(uri).unwrap();
            let schemes = SCHEMES.lock();
            let scheme = schemes.get(uri.scheme).unwrap();
            let resource = scheme
                .open(&uri, args[2] as _, unsafe { transmute(args[3]) })
                .unwrap();
            Task::current()
                .unwrap()
                .resources
                .lock()
                .insert(resource, scheme.scheme());
            Ok(unsafe { transmute(resource) })
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
            let schemes = SCHEMES.lock();
            let scheme = schemes
                .get(Task::current().unwrap().resources.lock()[&fd])
                .unwrap();
            let r = scheme.read(fd, buf).unwrap();
            Ok(unsafe { transmute(r) })
        }
        SchemeRequest::Write => {
            let fd = unsafe { transmute::<_, Resource>(args[1]) };
            let buf = unsafe { transmute::<_, &&[u8]>(args[2]) };
            let schemes = SCHEMES.lock();
            let scheme = schemes
                .get(Task::current().unwrap().resources.lock()[&fd])
                .unwrap();
            scheme.write(fd, buf).unwrap();
            Ok(0)
        }
    }
}
