// mod system;
use crate::task::*;
use crate::arch::*;
use crate::*;
use proton::KernelCall;
use core::marker::PhantomData;
use alloc::boxed::Box;
// use core::raw::TraitObject;
// const HANDLERS: [fn (m: &Message); KernelCall::COUNT] = [
//     system::task::fork,
//     system::task::exit,
//     system::task::sleep,
//     system::mem::map_physical_memory,
// ];

// pub trait KernelProcess {
//     type Kernel: AbstractKernel;
//     fn run() -> !;
// }

static mut KERNEL_PROCESS: Option<Box<dyn FnOnce(isize) -> !>> = None;

pub struct KernelProcess<K: AbstractKernel> {
    // handlers: [fn (m: &Message); KernelCall::COUNT],
    phantom: PhantomData<K>,
    // v: isize,
}

impl <K: AbstractKernel> KernelProcess <K> {
    // type Kernel = K;

    fn new() -> Self {
        Self {
            phantom: PhantomData,
        }
    }

    fn run(&mut self) -> ! {
        debug!(K: "Kernel process start");
        loop {
            debug_assert!(<K::Arch as AbstractArch>::Interrupt::is_enabled());
            // debug!(K: "Kernel process...");
            // let m = Message::receive(None);
            //     println!("Kernel received {:?}", m);
            //     HANDLERS[m.kind](&m);
            // }
        }
    }

    pub fn spawn() -> &'static mut Task::<K> {
        unsafe {
            KERNEL_PROCESS = Some(box |v| {
                debug!(K: "Kernel process arg={}", v);
                Self::new().run()
            });
        }
        let f = |v: isize| -> ! {
            debug!(K: "Kernel process arg={}", v);
            Self::new().run()
        };
        Task::<K>::create_kernel_task(unsafe {
            main
        })
    }
}

extern fn main(x: isize) -> ! {
    let func = unsafe { KERNEL_PROCESS.take().unwrap() };
    func(x)
}

// pub extern fn idle() -> ! {
//     loop {
//         unsafe { asm!("wfe"); }
//     }
// }