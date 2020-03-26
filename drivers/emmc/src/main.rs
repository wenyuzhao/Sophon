#![feature(asm)]
#![feature(format_args_nl)]
#![allow(dead_code)]
#![no_std]
#![no_main]

#[macro_use]
extern crate proton;
mod emmc;
mod constants;
mod fat;

use proton::Message;
use proton::driver::Driver;

pub struct EMMCDriver;

driver_entry!(EMMCDriver);

impl Driver for EMMCDriver {
    fn new() -> Self {
        emmc::EMMC::init().unwrap();
        fat::FAT::init().unwrap();
        fat::FAT::ls_root();
        Self
    }
    
    fn handle_message(&mut self, _m: &Message) {
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
