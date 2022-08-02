use crate::{
    arch::*,
    task::scheduler::{AbstractScheduler, SCHEDULER},
};
use alloc::vec;
use memory::page::{PageSize, Size4K};
use syscall::{ModuleRequest, Syscall};
use vfs::{Fd, VFSRequest};

use super::Proc;

pub fn init() {
    TargetArch::interrupt().set_syscall_handler(Some(box |syscall_id, a, b, c, d, e| {
        handle_syscall::<false>(syscall_id, a, b, c, d, e)
    }));
}

// =====================
// ===   Syscalls   ===
// =====================

fn handle_syscall<const PRIVILEGED: bool>(
    syscall_id: usize,
    a: usize,
    b: usize,
    c: usize,
    d: usize,
    e: usize,
) -> isize {
    let syscall: Syscall = unsafe { core::mem::transmute(syscall_id) };
    match syscall {
        Syscall::Log => log(a, b, c, d, e),
        Syscall::ModuleCall => module_request::<PRIVILEGED>(a, b, c, d, e),
        Syscall::Wait => {
            SCHEDULER.freeze_current_task();
            0
        }
        Syscall::Sbrk => Proc::current()
            .sbrk(a >> Size4K::LOG_BYTES)
            .map(|r| r.start.start().as_usize() as isize)
            .unwrap_or(-1),
        Syscall::Exec => exec(a, b, c, d, e),
    }
}

fn log(a: usize, _: usize, _: usize, _: usize, _: usize) -> isize {
    let string_pointer = a as *const &str;
    let s: &str = unsafe { &*string_pointer };
    print!("{}", s);
    0
}

fn module_request<const PRIVILEGED: bool>(
    a: usize,
    b: usize,
    c: usize,
    d: usize,
    e: usize,
) -> isize {
    let string_pointer = a as *const &str;
    let s: &str = unsafe { &*string_pointer };
    crate::modules::module_call(s, PRIVILEGED, [b, c, d, e])
}

fn exec(a: usize, _: usize, _: usize, _: usize, _: usize) -> isize {
    let path: &str = unsafe { &*(a as *const &str) };
    let mut elf = vec![];
    let fd = crate::modules::module_call("vfs", false, VFSRequest::Open(path).as_raw().as_buf());
    if fd < 0 {
        return -1;
    }
    let mut buf = [0u8; 256];
    loop {
        let size = crate::modules::module_call(
            "vfs",
            false,
            VFSRequest::Read(Fd(fd as _), &mut buf).as_raw().as_buf(),
        );
        if size > 0 {
            elf.extend_from_slice(&buf[0..size as usize]);
        } else if size < 0 {
            return -1;
        } else {
            break;
        }
    }
    let proc = Proc::spawn_user(elf);
    proc.id.0 as _
}
