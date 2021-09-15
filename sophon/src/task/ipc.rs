use core::intrinsics::transmute;

use super::{Message, Task, TaskId};
pub use crate::user::ipc::IPC;
use crate::{
    arch::*,
    task::uri::Uri,
    user::ipc::{Resource, SchemeServer},
};
use alloc::{boxed::Box, collections::BTreeMap};
use spin::Mutex;

pub fn init() {
    TargetArch::interrupt().set_handler(
        InterruptId::Soft,
        Some(box |ipc, a, b, c, d, e| {
            let ipc: IPC = unsafe { core::mem::transmute(ipc) };
            match ipc {
                IPC::Log => log(a, b, c, d, e),
                IPC::Send => send(a, b, c, d, e),
                IPC::Receive => receive(a, b, c, d, e),
            }
        }),
    );
}

// =====================
// ===   IPC Calls   ===
// =====================

fn log(a: usize, _: usize, _: usize, _: usize, _: usize) -> isize {
    let string_pointer = a as *const &str;
    let s: &str = unsafe { &*string_pointer };
    crate::log::_print(format_args!("{}", s));
    0
}

fn send(x1: usize, _: usize, _: usize, _: usize, _: usize) -> isize {
    let mut msg = unsafe { (*(x1 as *const Message)).clone() };
    msg.sender = Task::current().unwrap().id();
    match handle_scheme_request(msg) {
        Ok(()) => 0,
        Err(e) => e,
    }
    // Task::send_message(msg)
}

fn receive(x1: usize, _: usize, _: usize, _: usize, _: usize) -> isize {
    let from_id = unsafe {
        let id = core::mem::transmute::<_, isize>(x1);
        if id < 0 {
            None
        } else {
            Some(core::mem::transmute::<_, TaskId>(id))
        }
    };
    log!(
        "{:?} start receiving from {:?}",
        Task::current().unwrap().id(),
        from_id
    );
    Task::receive_message(from_id)
}

pub static SCHEMES: Mutex<BTreeMap<&'static str, Box<dyn SchemeServer + Send>>> =
    Mutex::new(BTreeMap::new());

fn handle_scheme_request(m: Message) -> Result<(), isize> {
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
