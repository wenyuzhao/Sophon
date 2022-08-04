use core::fmt;
use core::fmt::Write;

use vfs::Fd;

fn format(
    args: fmt::Arguments,
    f: impl Fn(&str) -> Result<(), fmt::Error>,
) -> Result<(), fmt::Error> {
    struct W<F>(F);
    impl<F: Fn(&str) -> Result<(), fmt::Error>> Write for W<F> {
        fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
            self.0(s)
        }
    }
    W(f).write_fmt(args)
}

#[doc(hidden)]
#[inline(never)]
pub fn _sys_log(args: fmt::Arguments) {
    let _ = format(args, |s| {
        syscall::log(s);
        Ok(())
    });
    syscall::log("\n");
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => ({
        // if $crate::IS_ENABLED {
            $crate::print::_sys_log(format_args!($($arg)*))
        // }
    });
}

#[doc(hidden)]
#[inline(never)]
pub fn _print(args: fmt::Arguments) {
    let _ = format(args, |s| match vfs::write(Fd::STDOUT, s.as_bytes()) {
        Ok(_) => Ok(()),
        Err(_) => Err(fmt::Error),
    });
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        $crate::print::_print(format_args!($($arg)*))
    });
}

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => ({
        $crate::print!($($arg)*);
        $crate::print!("\n");
    });
}
