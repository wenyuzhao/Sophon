#![feature(asm)]
#![feature(format_args_nl)]
#![no_std]
#![no_main]

#[macro_use]
extern crate proton;
mod emmc;
mod constants;

use proton::Message;
use proton::driver::Driver;

pub struct EMMCDriver;

driver_entry!(EMMCDriver);

impl Driver for EMMCDriver {
    fn new() -> Self {
        emmc::EMMC::init();
        Self
    }
    fn handle_message(&mut self, m: &Message) {
        unimplemented!()
    }
}

impl EMMCDriver {
    fn init(&mut self) {

    }
}

#[panic_handler]
fn panic(info: &::core::panic::PanicInfo) -> ! {
    log!("{}", info);
    loop {}
}
