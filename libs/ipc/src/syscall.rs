use crate::{Message, TaskId};
#[allow(unused)]
use core::arch::asm;
use core::intrinsics::transmute;

#[repr(usize)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Syscall {
    Log,
    Send,
    Receive,
    ModuleCall,
}

#[inline]
#[cfg(target_arch = "x86_64")]
pub(crate) fn syscall(_ipc: Syscall, _args: &[usize]) -> isize {
    unimplemented!()
}

#[inline]
#[cfg(target_arch = "aarch64")]
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
    syscall(Syscall::Log, &[&message as *const &str as usize]);
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

pub fn open(path: &str) -> isize {
    unsafe {
        let name = &"vfs" as *const &str;
        let kind = 0;
        let path = &path as *const &str;
        syscall(
            Syscall::ModuleCall,
            &[transmute(name), kind, transmute(path)],
        )
    }
}

pub fn read(fd: usize, mut buf: &mut [u8]) -> isize {
    unsafe {
        let name = &"vfs" as *const &str;
        let kind = 1;
        let buf = &mut buf as *mut &mut [u8];
        syscall(
            Syscall::ModuleCall,
            &[transmute(name), kind, fd, transmute(buf)],
        )
    }
}
