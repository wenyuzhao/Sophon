#[macro_use]
pub mod utils;
pub mod fork;

use crate::exception::ExceptionFrame;

#[repr(usize)]
#[derive(Debug, Eq, PartialEq)]
pub enum SysCall {
    Fork = 0x0,
    __MAX_SYSCALLS,
}

type SysCallHandler = fn (exception_frame: *mut ExceptionFrame) -> isize;

static SYSCALL_HANDLERS: [SysCallHandler; SysCall::__MAX_SYSCALLS as usize] = [
    fork::fork,
];

#[no_mangle]
pub extern fn handle_syscall(exception_frame: *mut ExceptionFrame) {
    debug!("exception_frame@{:?}", exception_frame as *mut _);
    let syscall_id: SysCall = unsafe { ::core::mem::transmute((*exception_frame).x0) };
    debug!("Syscall: {:?}", syscall_id);
    match syscall_id {
        SysCall::Fork => {
            // let parent_task = crate::task::Task::current().unwrap();
            // let child_task = parent_task.fork(exception_frame as *mut ExceptionFrame as usize);
            // unsafe { asm!("DSB SY") };
            unsafe { (*exception_frame).x0 = ::core::mem::transmute(fork::fork(exception_frame)) };
        }
        _ => unreachable!()
    }
}


