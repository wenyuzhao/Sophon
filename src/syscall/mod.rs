#[macro_use]
pub mod utils;
pub mod fork;

use crate::exception::ExceptionFrame;

#[repr(usize)]
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum SysCall {
    Fork = 0x0,
    #[allow(non_camel_case_types)] __MAX_SYSCALLS,
}

type SysCallHandler = fn (exception_frame: &mut ExceptionFrame) -> isize;

static SYSCALL_HANDLERS: [SysCallHandler; SysCall::__MAX_SYSCALLS as usize] = [
    fork::fork,
];


pub unsafe fn handle_syscall(exception_frame: &mut ExceptionFrame) {
    debug!("exception_frame@{:?}", exception_frame as *mut _);
    let syscall_id: SysCall = unsafe { ::core::mem::transmute((*exception_frame).x0) };
    debug!("Syscall: {:?}", syscall_id);
    let handler = SYSCALL_HANDLERS[syscall_id as usize];
    let result = handler(exception_frame);
    debug!("Syscall {:?} returned {:?}", syscall_id, result);
    exception_frame.x0 = ::core::mem::transmute(result);
}


