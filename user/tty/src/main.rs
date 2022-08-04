#![feature(format_args_nl)]
#![feature(default_alloc_error_handler)]
#![no_std]
#![no_main]

#[macro_use]
extern crate log;
extern crate alloc;

use alloc::{borrow::ToOwned, format, string::String, vec, vec::Vec};
// use core::arch::asm;
// use core::sync::atomic::{AtomicUsize, Ordering};
use heap::UserHeap;
use syscall::UserLogger;
use vfs::Fd;

#[global_allocator]
static ALLOCATOR: UserHeap = UserHeap::new();

struct TTY {}

impl TTY {
    fn new() -> Self {
        Self {}
    }

    fn write(&self, s: &str) {
        let _ = vfs::write(Fd::STDOUT, s.as_bytes()).unwrap();
    }

    fn prompt(&self) -> String {
        self.write("> ");
        self.readline()
    }

    fn read_byte(&self) -> u8 {
        let mut buf = [0u8; 1];
        let _len = vfs::read(Fd::STDIN, &mut buf).unwrap();
        let mut c = buf[0];
        if c == 127 {
            c = 8;
            buf[0] = c;
            let s = core::str::from_utf8(&buf).unwrap();
            print!("{} {}", s, s);
        } else {
            let s = core::str::from_utf8(&buf).unwrap();
            print!("{}", s);
        }
        c
    }

    fn readline(&self) -> String {
        let mut buf = vec![];
        loop {
            let c = self.read_byte();
            if c == 8 {
                buf.pop();
            } else if c == b'\n' {
                break;
            } else {
                buf.push(c);
            }
        }
        core::str::from_utf8(&buf).unwrap().to_owned()
    }

    pub fn run(&self) {
        self.write("[[Sophon TTY]]\n");
        loop {
            let cmd = self.prompt();
            // println!("{:?}", cmd);
            if cmd == "exit" {
                break;
            }
            let cmd = if !cmd.starts_with("/") && !cmd.starts_with(".") {
                format!("/bin/{}", cmd)
            } else {
                cmd
            };
            let segments = cmd
                .split(" ")
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>();
            let cmd = segments[0];
            let args = &segments[1..];
            syscall::exec(cmd, args);
        }
        self.write("Sophon TTY exited.\n");
    }
}

#[no_mangle]
pub extern "C" fn _start(_argc: isize, _argv: *const *const u8) -> isize {
    UserLogger::init();
    ALLOCATOR.init();
    let tty = TTY::new();
    tty.run();
    syscall::exit()
}

#[panic_handler]
fn panic(info: &::core::panic::PanicInfo) -> ! {
    log!("{}", info);
    syscall::exit();
}
