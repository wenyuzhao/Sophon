use core::intrinsics::transmute;

use crate::{Message, TaskId};

#[repr(usize)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Syscall {
    Log = 0,
    Send,
    Receive,
    SchemeRequest,
}

#[inline]
pub(crate) fn syscall(ipc: Syscall, args: &[usize]) -> isize {
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
        asm!("svc #0", in("x0") Syscall::Log as usize, in("x1") &message as *const &str);
    }
}

#[inline]
pub fn send(mut m: Message) {
    let ret = syscall(
        Syscall::Send,
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
        let ret = syscall(
            Syscall::Receive,
            &[transmute(from_task), transmute(&mut msg)],
        );
        assert!(ret == 0, "{:?}", ret);
        msg
    }
}
