use core::fmt;
use core::fmt::Write;
use spin::Mutex;
use crate::arch::*;
use crate::AbstractKernel;
use core::marker::PhantomData;

#[doc(hidden)]
#[inline(never)]
pub fn _print<K: AbstractKernel>(args: fmt::Arguments) {
    <K::Arch as AbstractArch>::Interrupt::uninterruptable(|| {
        let _guard = LOGGER_LOCK.lock();
        let mut write = Logger::<K::Arch>(PhantomData);
        write.write_fmt(args).unwrap();
    });
}

#[macro_export]
macro_rules! debug {
    ($arch: ty: $($arg:tt)*) => {{
        $crate::debug::_print::<$arch>(format_args_nl!($($arg)*))
    }};
}

static LOGGER_LOCK: Mutex<()> = Mutex::new(());

struct Logger<Arch: AbstractArch>(PhantomData<Arch>);

impl <Arch: AbstractArch> Logger<Arch> {
    fn putc(&self, c: char) {
        Arch::Logger::put(c)
    }
}

impl <Arch: AbstractArch> Write for Logger<Arch> {
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
