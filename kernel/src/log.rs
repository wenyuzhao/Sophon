use alloc::boxed::Box;
use core::fmt;
use core::fmt::Write;
use spin::Mutex;

#[allow(dead_code)]
pub static WRITER: Mutex<Option<Box<dyn Write + Send>>> = Mutex::new(None);

// pub struct Log;

// impl Write for Log {
//     fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
//         WRITER.lock().unwrap().write_str(s)
//     }
// }

#[doc(hidden)]
#[inline(never)]
pub fn _print(args: fmt::Arguments) {
    let mut writer = WRITER.lock();
    writer.as_mut().unwrap().write_fmt(args).unwrap();
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
