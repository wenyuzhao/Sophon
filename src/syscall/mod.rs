mod task;
mod log;
mod ipc;

use crate::exception::ExceptionFrame;

#[repr(usize)]
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum SysCall {
    Log = 0x0,
    Send,
    Receive,
    #[allow(non_camel_case_types)] __MAX_SYSCALLS,
}

type Handler = fn (exception_frame: &mut ExceptionFrame) -> isize;

macro_rules! handlers {
    ($($f: expr,)*) => { handlers![$($f),*] };
    ($($f: expr),*) => {[
        $(|ef: &mut ExceptionFrame| unsafe { ::core::mem::transmute($f(ef)) }),*
    ]};
}

static SYSCALL_HANDLERS: [Handler; SysCall::__MAX_SYSCALLS as usize] = handlers![
    log::log,
    ipc::send,
    ipc::receive,
];


pub unsafe fn handle_syscall(exception_frame: &mut ExceptionFrame) {
    let syscall_id: SysCall = unsafe { ::core::mem::transmute((*exception_frame).x0) };
    let handler = SYSCALL_HANDLERS[syscall_id as usize];
    let result = handler(exception_frame);
    exception_frame.x0 = ::core::mem::transmute(result);
}


