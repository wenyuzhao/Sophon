use super::proc::PROCESS_MANAGER;
use crate::INIT_FS;
use alloc::ffi::CString;
use alloc::vec::Vec;
use core::arch::asm;
use klib::task::Runnable;

/// The idle task.
///
/// The task scheduler should schedule this task when no other task is ready.
pub struct Idle;

impl Runnable for Idle {
    fn run(&mut self) -> ! {
        loop {
            unsafe {
                asm!("wfe");
            }
        }
    }
}

/// Main thread for the init process
pub struct Init {
    #[allow(unused)]
    args: Vec<CString>,
    elf: Vec<u8>,
}

impl Init {
    pub fn new() -> Self {
        let elf = INIT_FS
            .get()
            .unwrap()
            .get("/bin/init")
            .unwrap()
            .as_file()
            .unwrap()
            .to_vec();
        let args = Vec::new();
        Self { args, elf }
    }

    fn run_user(&mut self) -> ! {
        let proc = PROCESS_MANAGER.current_proc().unwrap();
        let elf = core::mem::take(&mut self.elf);
        let err = super::user::exec(proc, elf, &[]);
        panic!("Failed to exec init: {:?}", err);
    }
}

impl Runnable for Init {
    fn run(&mut self) -> ! {
        info!("init process running");
        if cfg!(sophon_test) {
            crate::utils::testing::run_kernel_tests_and_halt();
        }
        // Run the `init` user process
        self.run_user();
    }
}
