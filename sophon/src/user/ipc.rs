use crate::task::*;

#[repr(usize)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum IPC {
    Log = 0,
    Send,
    Receive,
}

#[inline]
pub fn log(message: &str) {
    unsafe {
        asm!("svc #0", in("x0") IPC::Log as usize, in("x1") &message as *const &str);
    }
}

#[inline]
pub fn send(mut m: Message) {
    let ret: isize;
    unsafe {
        asm!("svc #0", inout("x0") IPC::Send as usize => ret, in("x1") &mut m as *mut Message);
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
        asm!("svc #0", inout("x0") IPC::Receive as usize => ret, in("x1") from_task, in("x2") &mut msg as *mut Message);
        assert!(ret == 0, "{:?}", ret);
        msg
    }
}
