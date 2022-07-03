#![feature(format_args_nl)]
#![feature(default_alloc_error_handler)]
#![no_std]
#![no_main]

#[macro_use]
extern crate log;

use core::arch::asm;
use core::sync::atomic::{AtomicUsize, Ordering};
use heap::NoAlloc;
use ipc::log::UserLogger;
use ipc::scheme::{Args, Mode, Resource};

#[global_allocator]
static ALLOCATOR: NoAlloc = NoAlloc;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

extern "C" fn thread_start() {
    log!("thread_start");
    for _ in 0..10 {
        COUNTER.fetch_add(1, Ordering::SeqCst);
        for _ in 0..100000 {
            unsafe {
                asm!("");
            }
        }
        log!(" - {}", COUNTER.load(Ordering::SeqCst));
    }
    exit_thread();
}

fn exit_thread() {
    Resource::open("proc:/me/thread-exit", 0, Mode::ReadWrite)
        .unwrap()
        .write(&[])
        .unwrap();
}

fn spawn_thread(f: *const extern "C" fn()) {
    Resource::open("proc:/me/spawn-thread", 0, Mode::ReadWrite)
        .unwrap()
        .write(Args::new(f))
        .unwrap();
}

#[no_mangle]
pub extern "C" fn _start(_argc: isize, _argv: *const *const u8) -> isize {
    UserLogger::init();
    log!("Init process start (user mode)");
    let resource = Resource::open("scheme-test:/test", 0, Mode::ReadWrite).unwrap();
    let mut data = [0u8; 100];
    for _ in 0..1 {
        let len = resource.read(&mut data).unwrap();
        log!(
            "[init] read from scheme-test -> {:?}",
            core::str::from_utf8(&data[..len])
        );
        resource.write("hello, world").unwrap();
    }
    for _ in 0..10 {
        spawn_thread(thread_start as _);
    }
    loop {}
}

#[panic_handler]
fn panic(info: &::core::panic::PanicInfo) -> ! {
    log!("{}", info);
    loop {}
}
