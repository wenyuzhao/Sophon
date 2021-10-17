mod initfs;
mod proc;
mod system;
mod user;

use crate::task::{Proc, Task};
use alloc::{
    borrow::ToOwned,
    collections::BTreeMap,
    string::{String, ToString},
    sync::Arc,
};
use core::{
    intrinsics::transmute,
    slice, str,
    sync::atomic::{AtomicUsize, Ordering},
};
use interrupt::UninterruptibleMutex;
use ipc::scheme::{Resource, SchemeId, SchemeRequest, SchemeServer, Uri};
use spin::Mutex;

trait SchemeIdExt: Sized {
    fn alloc() -> SchemeId {
        static COUNTER: AtomicUsize = AtomicUsize::new(1);
        SchemeId(COUNTER.fetch_add(1, Ordering::SeqCst))
    }

    fn from_name(name: &str) -> Option<SchemeId> {
        SCHEME_IDS.lock().get(name).cloned()
    }
}

impl SchemeIdExt for SchemeId {}

pub static SCHEME_IDS: Mutex<BTreeMap<String, SchemeId>> = Mutex::new(BTreeMap::new());
pub static SCHEMES: Mutex<BTreeMap<SchemeId, Arc<dyn SchemeServer + Send + Sync>>> =
    Mutex::new(BTreeMap::new());

fn register_kernel_scheme(scheme: Arc<dyn SchemeServer + Send + Sync>) {
    let id = SchemeId::alloc();
    SCHEME_IDS.lock().insert(scheme.name().to_owned(), id);
    SCHEMES.lock().insert(id, scheme);
}

pub fn register_kernel_schemes() {
    register_kernel_scheme(Arc::new(system::SystemScheme::new()));
    register_kernel_scheme(Arc::new(initfs::InitFSScheme::new()));
    register_kernel_scheme(Arc::new(proc::ProcScheme::new()));
}

fn get_scheme_by_id(id: SchemeId) -> Arc<dyn SchemeServer> {
    SCHEMES.lock().get(&id).unwrap().clone()
}

pub fn handle_scheme_request(args: &[usize; 5]) -> Result<isize, isize> {
    match unsafe { transmute::<_, SchemeRequest>(args[0]) } {
        SchemeRequest::Register => {
            let name = unsafe { transmute::<_, &&str>(args[1]) };
            register_kernel_scheme(Arc::new(user::UserScheme::new(
                name.to_string(),
                Task::current().id,
            )));
            Ok(0)
        }
        SchemeRequest::Open => {
            let uri = unsafe {
                let uri_ptr = transmute::<_, *const u8>(args[1]);
                let uri_len = transmute::<_, usize>(args[2]);
                let uri_str = str::from_utf8_unchecked(slice::from_raw_parts(uri_ptr, uri_len));
                Uri::new(uri_str).unwrap()
            };
            let scheme_id = SchemeId::from_name(uri.scheme).unwrap();
            let scheme = get_scheme_by_id(scheme_id);
            let resource = scheme
                .open(&uri, args[3] as _, unsafe { transmute(args[4]) })
                .unwrap();
            Proc::current()
                .resources
                .lock_uninterruptible()
                .insert(resource, scheme_id);
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
            let buf = unsafe {
                let buf_ptr = transmute::<_, *mut u8>(args[2]);
                let buf_len = transmute::<_, usize>(args[3]);
                slice::from_raw_parts_mut(buf_ptr, buf_len)
            };
            let scheme = get_scheme_by_id(Proc::current().resources.lock_uninterruptible()[&fd]);
            let r = scheme.read(fd, buf).unwrap();
            Ok(unsafe { transmute(r) })
        }
        SchemeRequest::Write => {
            let fd = unsafe { transmute::<_, Resource>(args[1]) };
            let buf = unsafe {
                let buf_ptr = transmute::<_, *const u8>(args[2]);
                let buf_len = transmute::<_, usize>(args[3]);
                slice::from_raw_parts(buf_ptr, buf_len)
            };
            let scheme = get_scheme_by_id(Proc::current().resources.lock_uninterruptible()[&fd]);
            scheme.write(fd, buf).unwrap();
            Ok(0)
        }
    }
}
