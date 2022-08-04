#![feature(format_args_nl)]
#![feature(default_alloc_error_handler)]
#![no_std]
#![no_main]

#[macro_use]
extern crate log;

// use core::arch::asm;
// use core::sync::atomic::{AtomicUsize, Ordering};
use heap::UserHeap;
use syscall::UserLogger;

#[global_allocator]
static ALLOCATOR: UserHeap = UserHeap::new();

// static COUNTER: AtomicUsize = AtomicUsize::new(0);

// extern "C" fn thread_start() {
//     log!("thread_start");
//     for _ in 0..10 {
//         COUNTER.fetch_add(1, Ordering::SeqCst);
//         for _ in 0..100000 {
//             unsafe {
//                 asm!("");
//             }
//         }
//         log!(" - {}", COUNTER.load(Ordering::SeqCst));
//     }
//     exit_thread();
// }

// fn exit_thread() {
//     Resource::open("proc:/me/thread-exit", 0, Mode::ReadWrite)
//         .unwrap()
//         .write(&[])
//         .unwrap();
// }

// fn spawn_thread(f: *const extern "C" fn()) {
//     Resource::open("proc:/me/spawn-thread", 0, Mode::ReadWrite)
//         .unwrap()
//         .write(Args::new(f))
//         .unwrap();
// }

#[no_mangle]
pub extern "C" fn _start(_argc: isize, _argv: *const *const u8) -> isize {
    UserLogger::init();
    log!("Init process start...");
    log!("Test fs read...");
    let file = vfs::open("/etc/hello.txt").unwrap();
    let mut buf = [0u8; 32];
    let len = vfs::read(file, &mut buf).unwrap();
    let s = core::str::from_utf8(&buf[0..len]);
    log!("read: {:?}", s);
    log!("Launch tty...");
    syscall::exec("/bin/tty", &[]);
    syscall::exit()
}

#[panic_handler]
fn panic(info: &::core::panic::PanicInfo) -> ! {
    log!("{}", info);
    syscall::exit();
}
