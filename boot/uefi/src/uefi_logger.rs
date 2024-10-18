use core::fmt;
use log::Logger;
use uefi::CStr16;

pub struct UEFILogger;

impl UEFILogger {
    pub fn init() {
        log::init(&UEFILogger)
    }
}

impl Logger for UEFILogger {
    #[inline]
    fn log(&self, s: &str) -> Result<(), fmt::Error> {
        uefi::system::with_stdout(|out| {
            for c in s.chars() {
                if c == '\n' {
                    let v = ['\r' as u16, 0];
                    let _ = out
                        .output_string(CStr16::from_u16_with_nul(&v).ok().unwrap())
                        .unwrap();
                }
                let v = [c as u16, 0];
                let _ = out
                    .output_string(CStr16::from_u16_with_nul(&v).ok().unwrap())
                    .unwrap();
            }
        });
        Ok(())
    }
}
