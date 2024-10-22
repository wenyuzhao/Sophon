use super::proc::PROCESS_MANAGER;
use super::sched::SCHEDULER;
use crate::arch::Arch;
use crate::arch::TargetArch;
use memory::page::{PageSize, Size4K};
use syscall::Syscall;

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
        Syscall::Fork => fork(a, b, c, d, e),
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

fn fork(_: usize, _: usize, _: usize, _: usize, _: usize) -> isize {
    let proc = PROCESS_MANAGER.current_proc().unwrap();
    let child = PROCESS_MANAGER.fork(proc);
    // we are still the parent
    return child.id.0 as isize;
}

fn exec(a: usize, b: usize, _: usize, _: usize, _: usize) -> isize {
    let path: &str = unsafe { &*(a as *const &str) };
    let args: &[&str] = unsafe { &*(b as *const &[&str]) };
    PROCESS_MANAGER.exec(path, args)
}

fn exit(_: usize, _: usize, _: usize, _: usize, _: usize) -> isize {
    PROCESS_MANAGER.exit_current_proc();
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
