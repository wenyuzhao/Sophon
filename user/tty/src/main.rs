#![feature(default_alloc_error_handler)]
#![no_std]
#![no_main]

extern crate alloc;

#[macro_use]
extern crate user;

use alloc::{borrow::ToOwned, format, string::String, vec, vec::Vec};
use user::sys::Fd;

struct TTY {}

impl TTY {
    fn new() -> Self {
        Self {}
    }

    fn prompt(&self) -> String {
        let cwd = user::sys::cwd().unwrap();
        print!("{} $ ", cwd);
        self.readline()
    }

    fn read_byte(&self, first: bool) -> u8 {
        let mut buf = [0u8; 1];
        let _len = user::sys::read(Fd::STDIN, &mut buf).unwrap();
        let mut c = buf[0];
        if c == 127 {
            c = 8;
            buf[0] = c;
            let s = core::str::from_utf8(&buf).unwrap();
            if !first {
                print!("{} {}", s, s);
            }
        } else {
            let s = core::str::from_utf8(&buf).unwrap();
            print!("{}", s);
        }
        c
    }

    fn readline(&self) -> String {
        let mut buf = vec![];
        loop {
            let c = self.read_byte(buf.is_empty());
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

    fn is_internal_cmd(&self, cmd: &str) -> bool {
        ["exit", "cd", "pwd"].iter().any(|&x| x == cmd)
    }

    fn exec_internal_cmd(&self, cmd: &str, args: &[&str]) {
        match cmd {
            "exit" => {
                println!("Sophon TTY exited.");
                user::sys::halt(0);
            }
            "cd" => {
                if args.len() == 1 {
                    match user::sys::chdir(&args[0]) {
                        Ok(_) => {}
                        Err(_) => {
                            println!("cd: no such file or directory");
                        }
                    };
                } else {
                    println!("Usage: cd <path>");
                }
            }
            "pwd" => {
                let cwd = user::sys::cwd().unwrap();
                println!("{}", cwd);
            }
            _ => unreachable!(),
        }
    }

    fn exec_external_cmd(&self, cmd: &str, args: &[&str]) {
        let cmd = if !cmd.starts_with("/") && !cmd.starts_with(".") {
            format!("/bin/{}", cmd)
        } else {
            cmd.to_owned()
        };
        if user::sys::exec(&cmd, args) == -1 {
            // FIXME
            println!("ERROR: command not found");
        }
    }

    pub fn run(&self) {
        println!("[[Sophon TTY]]");
        loop {
            let cmd = self.prompt();
            // println!("{:?}", cmd);

            let segments = cmd
                .split(" ")
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>();
            let cmd = segments[0];
            let args = &segments[1..];
            if self.is_internal_cmd(cmd) {
                self.exec_internal_cmd(cmd, args)
            } else {
                self.exec_external_cmd(cmd, args)
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn _start(_argc: isize, _argv: *const *const u8) -> isize {
    let tty = TTY::new();
    tty.run();
    user::sys::exit()
}
