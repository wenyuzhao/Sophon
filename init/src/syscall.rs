
#[repr(usize)]
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum SysCall {
    Fork = 0x0,
    Log,
    Exit,
}

pub unsafe fn syscall(id: SysCall, args: [usize; 6]) -> isize {
    let ret: isize;
    asm! {
        "svc #0"
        ::"{x0}"(id as usize)
          "{x1}"(args[0]), "{x2}"(args[1]), "{x3}"(args[2]),
          "{x4}"(args[3]), "{x5}"(args[4]), "{x6}"(args[5])
        :"x0" "x1" "x2" "x3" "x4" "x5" "x6" "memory"
    }
    asm!("": "={x0}"(ret));
    ret
}

#[macro_export]
macro_rules! syscall {
    ($id: expr, $a: expr, $b: expr, $c: expr, $d: expr, $e: expr, $f: expr) => ({
        use core::mem::transmute as t;
        unsafe {
            $crate::syscall::syscall($id, [t($a), t($b), t($c), t($d), t($e), t($f)])
        }
    });
    ($id: expr, $a: expr, $b: expr, $c: expr, $d: expr, $e: expr) => (syscall!($id, $a, $b, $c, $d, $e, 0usize));
    ($id: expr, $a: expr, $b: expr, $c: expr, $d: expr) => (syscall!($id, $a, $b, $c, $d, 0usize, 0usize));
    ($id: expr, $a: expr, $b: expr, $c: expr) => (syscall!($id, $a, $b, $c, 0usize, 0usize, 0usize));
    ($id: expr, $a: expr, $b: expr) => (syscall!($id, $a, $b, 0usize, 0usize, 0usize, 0usize));
    ($id: expr, $a: expr) => (syscall!($id, $a, 0usize, 0usize, 0usize, 0usize, 0usize));
    ($id: expr) => (syscall!($id, 0usize, 0usize, 0usize, 0usize, 0usize, 0usize));
}