use core::intrinsics::transmute;

use alloc::{boxed::Box, collections::BTreeMap};
use spin::Mutex;

use crate::{
    task::{uri::Uri, Message, Task},
    user::ipc::{Resource, Result as IoResult, SchemeServer},
};

pub static SCHEMES: Mutex<BTreeMap<&'static str, Box<dyn SchemeServer + Send>>> =
    Mutex::new(BTreeMap::new());

struct SystemSchemeServer {}

impl SchemeServer for SystemSchemeServer {
    fn scheme(&self) -> &'static str {
        "system"
    }
    fn open(&self, _uri: &Uri) -> IoResult<Resource> {
        log!("SystemSchemeServer 0");
        Ok(Resource(0))
    }
    fn read(&self, _fd: Resource, buf: &mut [u8]) -> IoResult<()> {
        buf[0] += 1;
        Ok(())
    }
    fn write(&self, _fd: Resource, _buf: &[u8]) -> IoResult<()> {
        unimplemented!()
    }
}

pub fn register_kernel_schemes() {
    SCHEMES.lock().insert("system", box SystemSchemeServer {});
}

pub fn handle_scheme_request(m: Message) -> Result<(), isize> {
    let args = m.get_data::<[u64; 6]>();
    match args[0] {
        0 => {
            let uri = unsafe { transmute::<_, &&str>(args[1]) };
            let uri = Uri::new(uri).unwrap();
            let schemes = SCHEMES.lock();
            let scheme = schemes.get(uri.scheme).unwrap();
            let resource = scheme.open(&uri).unwrap();
            let result = unsafe { transmute::<_, &mut Resource>(args[2]) };
            Task::current()
                .unwrap()
                .resources
                .lock()
                .insert(resource, scheme.scheme());
            *result = resource;
            Ok(())
        }
        1 => {
            let fd = unsafe { transmute::<_, Resource>(args[1]) };
            let buf = unsafe { transmute::<_, &mut &mut [u8]>(args[2]) };
            let schemes = SCHEMES.lock();
            let scheme = schemes
                .get(Task::current().unwrap().resources.lock()[&fd])
                .unwrap();
            scheme.read(fd, buf).unwrap();
            Ok(())
        }
        2 => {
            let fd = unsafe { transmute::<_, Resource>(args[1]) };
            let buf = unsafe { transmute::<_, &&[u8]>(args[2]) };
            let schemes = SCHEMES.lock();
            let scheme = schemes
                .get(Task::current().unwrap().resources.lock()[&fd])
                .unwrap();
            scheme.write(fd, buf).unwrap();
            Ok(())
        }
        _ => unimplemented!(),
    }
}
