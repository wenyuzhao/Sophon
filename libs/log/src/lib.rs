#![no_std]

use core::fmt;
use core::fmt::Write;

pub const IS_ENABLED: bool = cfg!(not(feature = "disable"));

pub fn format(
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

pub fn log_fmt(logger: &impl Logger, args: fmt::Arguments) -> Result<(), fmt::Error> {
    format(args, |s| logger.log(s))
}

pub trait Logger: Send {
    fn log(&self, message: &str) -> Result<(), fmt::Error>;
    fn log_fmt(&self, args: fmt::Arguments) -> Result<(), fmt::Error> {
        format(args, |s| self.log(s))
    }
}

impl<T: Logger + Sync> Logger for &T {
    fn log(&self, message: &str) -> Result<(), fmt::Error> {
        (**self).log(message)
    }
}

static mut LOGGER: Option<&'static dyn Logger> = None;

pub fn init(logger: &'static dyn Logger) {
    unsafe {
        LOGGER = Some(logger);
    }
}

#[doc(hidden)]
#[inline(never)]
pub fn _print(args: fmt::Arguments) {
    unsafe { LOGGER.as_mut().map(|logger| logger.log_fmt(args).unwrap()) };
}

#[macro_export]
macro_rules! log {
    (noeol: $($arg:tt)*) => ({
        if $crate::IS_ENABLED {
            $crate::_print(format_args!($($arg)*))
        }
    });
    ($($arg:tt)*) => ({
        if $crate::IS_ENABLED {
            $crate::_print(format_args_nl!($($arg)*))
        }
    });
}
