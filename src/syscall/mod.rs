mod task;
mod log;
mod ipc;

use crate::arch::*;

#[repr(usize)]
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum SysCall {
    Log = 0x0,
    Send,
    Receive,
    #[allow(non_camel_case_types)] __MAX_SYSCALLS,
}

type Handler = fn (x0: usize, x1: usize, x2: usize, x3: usize, x4: usize, x5: usize) -> isize;

macro_rules! handlers {
    ($($f: expr,)*) => { handlers![$($f),*] };
    ($($f: expr),*) => {[
        $(|x0: usize, x1: usize, x2: usize, x3: usize, x4: usize, x5: usize| unsafe { ::core::mem::transmute($f(x0, x1, x2, x3, x4, x5)) }),*
    ]};
}

static SYSCALL_HANDLERS: [Handler; SysCall::__MAX_SYSCALLS as usize] = handlers![
    log::log,
    ipc::send,
    ipc::receive,
];

pub fn init() {
    Target::Interrupt::set_handler(InterruptId::Soft, Some(handle_syscall));
}

fn handle_syscall(x0: usize, x1: usize, x2: usize, x3: usize, x4: usize, x5: usize) -> isize {
    let syscall_id: SysCall = unsafe { ::core::mem::transmute(x0) };
    let handler = SYSCALL_HANDLERS[syscall_id as usize];
    handler(x0, x1, x2, x3, x4, x5)
}


