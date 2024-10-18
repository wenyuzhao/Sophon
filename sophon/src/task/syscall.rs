use super::runnables::UserTask;
use crate::arch::Arch;
use crate::modules::PROCESS_MANAGER;
use crate::{arch::TargetArch, modules::SCHEDULER};
use alloc::vec;
use memory::page::{PageSize, Size4K};
use syscall::Syscall;
use vfs::{Fd, VFSRequest};

// =====================
// ===   Syscalls   ===
// =====================

pub fn handle_syscall<const PRIVILEGED: bool>(
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
            SCHEDULER.sleep();
            0
        }
        Syscall::Sbrk => crate::memory::utils::sbrk(
            PROCESS_MANAGER.current_proc().unwrap(),
            a >> Size4K::LOG_BYTES,
        )
        .map(|r| r.start.start().as_usize() as isize)
        .unwrap_or(-1),
        Syscall::Exec => exec(a, b, c, d, e),
        Syscall::Exit => exit(a, b, c, d, e),
        Syscall::ThreadExit => thread_exit(a, b, c, d, e),
        Syscall::Halt => halt(a, b, c, d, e),
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
    crate::modules::raw_module_call(s, PRIVILEGED, [b, c, d, e])
}

fn exec(a: usize, b: usize, _: usize, _: usize, _: usize) -> isize {
    let path: &str = unsafe { &*(a as *const &str) };
    let args: &[&str] = unsafe { &*(b as *const &[&str]) };
    let mut elf = vec![];
    let fd = crate::modules::module_call("vfs", false, &VFSRequest::Open(path));
    if fd < 0 {
        return -1;
    }
    let mut buf = [0u8; 256];
    loop {
        let size =
            crate::modules::module_call("vfs", false, &VFSRequest::Read(Fd(fd as _), &mut buf));
        if size > 0 {
            elf.extend_from_slice(&buf[0..size as usize]);
        } else if size < 0 {
            return -1;
        } else {
            break;
        }
    }
    crate::modules::module_call("vfs", false, &VFSRequest::Close(Fd(fd as _)));
    let proc = UserTask::spawn_user_process(elf, args);
    proc.wait_for_completion();
    proc.id().0 as _
}

fn exit(_: usize, _: usize, _: usize, _: usize, _: usize) -> isize {
    PROCESS_MANAGER.current_proc().unwrap().exit();
    SCHEDULER.schedule()
}

fn thread_exit(_: usize, _: usize, _: usize, _: usize, _: usize) -> isize {
    // Note: `Task::current()` must be dropped before calling `schedule`.
    PROCESS_MANAGER.end_current_task();
    SCHEDULER.schedule()
}
fn halt(a: usize, _: usize, _: usize, _: usize, _: usize) -> isize {
    TargetArch::halt(a as _)
}
