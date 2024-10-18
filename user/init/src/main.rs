#![no_std]
#![no_main]

#[macro_use]
extern crate user;

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
    println!("Init process start...");
    println!("Launch tty...");
    user::sys::exec("/bin/tty", &[]);
    user::sys::exit()
}
