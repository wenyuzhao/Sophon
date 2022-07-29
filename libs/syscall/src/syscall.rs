#[allow(unused)]
use core::arch::asm;
use core::intrinsics::transmute;

#[repr(usize)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Syscall {
    Log,
    ModuleCall,
}

#[inline]
#[cfg(target_arch = "x86_64")]
pub(crate) fn syscall(_syscall: Syscall, _args: &[usize]) -> isize {
    unimplemented!()
}

#[inline]
#[cfg(target_arch = "aarch64")]
pub(crate) fn syscall(syscall: Syscall, args: &[usize]) -> isize {
    debug_assert!(args.len() <= 6);
    let a: usize = args.get(0).cloned().unwrap_or(0);
    let b: usize = args.get(1).cloned().unwrap_or(0);
    let c: usize = args.get(2).cloned().unwrap_or(0);
    let d: usize = args.get(3).cloned().unwrap_or(0);
    let e: usize = args.get(4).cloned().unwrap_or(0);
    let ret: isize;
    unsafe {
        asm!("svc #0",
            inout("x0") syscall as usize => ret,
            in("x1") a, in("x2") b, in("x3") c, in("x4") d, in("x5") e,
        );
    }
    ret
}

#[inline]
pub fn log(message: &str) {
    syscall(Syscall::Log, &[&message as *const &str as usize]);
}

pub fn open(path: &str) -> isize {
    unsafe {
        let name = &"vfs" as *const &str;
        let kind = 0;
        let path = &path as *const &str;
        syscall(
            Syscall::ModuleCall,
            &[transmute(name), kind, transmute(path)],
        )
    }
}

pub fn read(fd: usize, mut buf: &mut [u8]) -> isize {
    unsafe {
        let name = &"vfs" as *const &str;
        let kind = 1;
        let buf = &mut buf as *mut &mut [u8];
        syscall(
            Syscall::ModuleCall,
            &[transmute(name), kind, fd, transmute(buf)],
        )
    }
}
