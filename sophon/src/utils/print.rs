use core::{
    fmt::Write,
    sync::atomic::{AtomicBool, Ordering},
};

static mut LOGGER: Option<&'static mut dyn Write> = None;
static IS_LOG_LOGGER_SET: AtomicBool = AtomicBool::new(false);

pub fn init(logger: &'static mut dyn Write) {
    unsafe {
        LOGGER = Some(logger);
    }
    if !IS_LOG_LOGGER_SET.swap(true, Ordering::SeqCst) {
        log::set_logger(&LOG_LOGGER).unwrap();
        log::set_max_level(log::LevelFilter::Trace);
    }
}

#[doc(hidden)]
#[inline(never)]
#[allow(static_mut_refs)]
pub fn _print(args: core::fmt::Arguments) {
    unsafe {
        LOGGER
            .as_mut()
            .map(|logger| logger.write_fmt(args).unwrap())
    };
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ({
        $crate::utils::print::_print(format_args!($($arg)*))
    });
}

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => ({
        $crate::utils::print::_print(format_args_nl!($($arg)*))
    });
}

struct KernelLogLogger;

static LOG_LOGGER: KernelLogLogger = KernelLogLogger;

impl log::Log for KernelLogLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    #[inline]
    #[allow(static_mut_refs)]
    fn log(&self, record: &log::Record) {
        let out = unsafe { LOGGER.as_mut().unwrap() };

        writeln!(
            out,
            "[{}][{}] {}",
            record.level(),
            record.target(),
            record.args()
        )
        .unwrap();
    }

    fn flush(&self) {}
}
