use crate::arch::*;
use syscall::Syscall;

pub fn init() {
    TargetArch::interrupt().set_handler(
        InterruptId::Soft,
        Some(box |syscall_id, a, b, c, d, e| {
            let syscall: Syscall = unsafe { core::mem::transmute(syscall_id) };
            match syscall {
                Syscall::Log => log(a, b, c, d, e),
                Syscall::ModuleCall => module_request(a, b, c, d, e),
            }
        }),
    );
}

// =====================
// ===   Syscalls   ===
// =====================

fn log(a: usize, _: usize, _: usize, _: usize, _: usize) -> isize {
    let string_pointer = a as *const &str;
    let s: &str = unsafe { &*string_pointer };
    print!("{}", s);
    0
}

fn module_request(a: usize, b: usize, c: usize, d: usize, e: usize) -> isize {
    let string_pointer = a as *const &str;
    let s: &str = unsafe { &*string_pointer };
    crate::modules::module_call(s, [b, c, d, e])
}
