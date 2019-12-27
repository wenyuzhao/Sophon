use core::fmt;
use core::fmt::Write;
use spin::Mutex;
use crate::arch::*;

#[doc(hidden)]
#[inline(never)]
pub fn _print(args: fmt::Arguments) {
    Target::Interrupt::uninterruptable(|| {
        let mut write = LOGGER.lock();
        write.write_fmt(args).unwrap();
    });
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {{
        $crate::debug::_print(format_args!($($arg)*))
    }};
}

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {{
        $crate::debug::_print(format_args_nl!($($arg)*))
    }};
}

static LOGGER: Mutex<Logger> = Mutex::new(Logger);

struct Logger;

impl Logger {
    fn putc(&self, c: char) {
        Target::Logger::put(c)
    }
}

impl Write for Logger {
    fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
        for c in s.chars() {
            if c == '\n' {
                self.putc('\r')
            }
            self.putc(c)
        }
        Ok(())
    }
}
