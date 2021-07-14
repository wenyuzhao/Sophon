use crate::arch::{Arch, TargetArch};
use alloc::boxed::Box;
use core::fmt;
use core::fmt::Write;
use spin::Mutex;

#[allow(dead_code)]
pub static WRITER: Mutex<Option<Box<dyn Write + Send>>> = Mutex::new(None);

#[doc(hidden)]
#[inline(never)]
pub fn _print(args: fmt::Arguments) {
    TargetArch::uninterruptable(|| {
        let mut writer = WRITER.lock();
        if let Some(writer) = writer.as_mut() {
            writer.write_fmt(args).unwrap();
        }
    });
}

#[macro_export]
macro_rules! log {
    (noeol: $($arg:tt)*) => ({
        $crate::log::_print(format_args!($($arg)*))
    });
    ($($arg:tt)*) => ({
        $crate::log::_print(format_args_nl!($($arg)*))
    });
}
