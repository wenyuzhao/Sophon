use crate::arch::Arch;
use crate::utils::locks::{RawCondvar, RawMutex};
use crate::{arch::TargetArch, modules::SCHEDULER};
use alloc::boxed::Box;
use alloc::vec;
use memory::page::{PageSize, Size4K};
use syscall::Syscall;
use vfs::{Fd, VFSRequest};

use super::Proc;

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
        Syscall::Sbrk => Proc::current()
            .sbrk(a >> Size4K::LOG_BYTES)
            .map(|r| r.start.start().as_usize() as isize)
            .unwrap_or(-1),
        Syscall::Exec => exec(a, b, c, d, e),
        Syscall::Exit => exit(a, b, c, d, e),
        Syscall::Halt => halt(a, b, c, d, e),
        Syscall::MutexCreate => mutex_create::<PRIVILEGED>(a, b, c, d, e),
        Syscall::MutexLock => mutex_lock(a, b, c, d, e),
        Syscall::MutexUnlock => mutex_unlock(a, b, c, d, e),
        Syscall::MutexDestroy => mutex_destroy(a, b, c, d, e),
        Syscall::CondvarCreate => condvar_create::<PRIVILEGED>(a, b, c, d, e),
        Syscall::CondvarWait => condvar_wait(a, b, c, d, e),
        Syscall::CondvarNotifyAll => condvar_notify_all(a, b, c, d, e),
        Syscall::CondvarDestroy => condvar_destroy(a, b, c, d, e),
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
    let proc = Proc::spawn_user(elf, args);
    let mut live = proc.live.lock();
    while *live {
        live = proc.live.wait(live);
    }
    proc.id.0 as _
}

fn exit(_: usize, _: usize, _: usize, _: usize, _: usize) -> isize {
    Proc::current().exit();
    SCHEDULER.schedule()
}

fn halt(a: usize, _: usize, _: usize, _: usize, _: usize) -> isize {
    TargetArch::halt(a as _)
}

fn mutex_create<const PRIVILEGED: bool>(_: usize, _: usize, _: usize, _: usize, _: usize) -> isize {
    let mutex = Box::leak(box RawMutex::new()) as *mut RawMutex;
    if !PRIVILEGED {
        Proc::current().locks.lock().push(mutex);
    }
    mutex as _
}

fn mutex_lock(a: usize, _: usize, _: usize, _: usize, _: usize) -> isize {
    let mutex = unsafe { &*(a as *const RawMutex) };
    mutex.lock();
    0
}

fn mutex_unlock(a: usize, _: usize, _: usize, _: usize, _: usize) -> isize {
    let mutex = unsafe { &*(a as *const RawMutex) };
    mutex.unlock();
    0
}

fn mutex_destroy(a: usize, _: usize, _: usize, _: usize, _: usize) -> isize {
    let mutex = a as *mut RawMutex;
    let proc = Proc::current();
    let mut locks = proc.locks.lock();
    if locks.drain_filter(|x| *x == mutex).count() > 0 {
        let _boxed = unsafe { Box::from_raw(mutex) };
    }
    0
}

fn condvar_create<const PRIVILEGED: bool>(
    _: usize,
    _: usize,
    _: usize,
    _: usize,
    _: usize,
) -> isize {
    let condvar = Box::leak(box RawCondvar::new()) as *mut RawCondvar;
    if !PRIVILEGED {
        Proc::current().cvars.lock().push(condvar);
    }
    condvar as _
}

fn condvar_wait(a: usize, b: usize, _: usize, _: usize, _: usize) -> isize {
    let condvar = unsafe { &*(a as *const RawCondvar) };
    let mutex = unsafe { &*(b as *const RawMutex) };
    condvar.wait(mutex);
    0
}

fn condvar_notify_all(a: usize, _: usize, _: usize, _: usize, _: usize) -> isize {
    let condvar = unsafe { &*(a as *const RawCondvar) };
    condvar.notify_all();
    0
}

fn condvar_destroy(a: usize, _: usize, _: usize, _: usize, _: usize) -> isize {
    let cvar = a as *mut RawCondvar;
    let proc = Proc::current();
    let mut cvars = proc.cvars.lock();
    if cvars.drain_filter(|x| *x == cvar).count() > 0 {
        let _boxed = unsafe { Box::from_raw(cvar) };
    }
    0
}
