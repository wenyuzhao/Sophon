use super::{Message, Task, TaskId};
use crate::{arch::*, schemes::handle_scheme_request};
use ipc::syscall::Syscall;

pub fn init() {
    TargetArch::interrupt().set_handler(
        InterruptId::Soft,
        Some(box |ipc, a, b, c, d, e| {
            let syscall: Syscall = unsafe { core::mem::transmute(ipc) };
            match syscall {
                Syscall::Log => log(a, b, c, d, e),
                Syscall::Send => send(a, b, c, d, e),
                Syscall::Receive => receive(a, b, c, d, e),
                Syscall::SchemeRequest => scheme_request(a, b, c, d, e),
                Syscall::ModuleCall => module_request(a, b, c, d, e),
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
    print!("{}", s);
    0
}

fn send(x1: usize, _: usize, _: usize, _: usize, _: usize) -> isize {
    let mut msg = unsafe { (*(x1 as *const Message)).clone() };
    msg.sender = Task::current().id;
    Task::send_message(msg)
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
        Task::current().id,
        from_id
    );
    Task::receive_message(from_id)
}

fn scheme_request(a: usize, b: usize, c: usize, d: usize, e: usize) -> isize {
    handle_scheme_request(&[a, b, c, d, e]).unwrap_or_else(|e| e)
}

fn module_request(a: usize, b: usize, c: usize, d: usize, e: usize) -> isize {
    let string_pointer = a as *const &str;
    let s: &str = unsafe { &*string_pointer };
    crate::modules::module_call(s, [b, c, d, e])
}
