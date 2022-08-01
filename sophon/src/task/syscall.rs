use crate::{
    arch::*,
    task::scheduler::{AbstractScheduler, SCHEDULER},
};
use memory::page::{PageSize, Size4K};
use syscall::Syscall;

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
