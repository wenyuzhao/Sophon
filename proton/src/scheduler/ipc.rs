pub use crate::task::Message;
use crate::{
    arch::*,
    task::{Task, TaskId},
};

#[repr(usize)]
pub enum IPC {
    Log = 0,
    Send,
    Receive,
}

impl IPC {
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

    #[inline]
    pub fn log(message: &str) {
        unsafe {
            asm!("svc #0", in("x0") Self::Log as usize, in("x1") &message as *const &str);
        }
    }

    #[inline]
    pub fn send(mut m: Message) {
        let ret: isize;
        unsafe {
            asm!("svc #0", inout("x0") Self::Send as usize => ret, in("x1") &mut m as *mut Message);
        }
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
            let ret: isize;
            asm!("svc #0", inout("x0") Self::Receive as usize => ret, in("x1") from_task, in("x2") &mut msg as *mut Message);
            assert!(ret == 0, "{:?}", ret);
            msg
        }
    }
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
    let current_task = Task::current().unwrap();
    msg.sender = current_task.id();
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
        Task::current().unwrap().id(),
        from_id
    );
    Task::receive_message(from_id)
}
