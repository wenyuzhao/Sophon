pub mod proc;
pub mod runnables;
pub mod syscall;

pub use self::proc::*;
use crate::modules::PROCESS_MANAGER;
use ::proc::Runnable;
pub use ::proc::{ProcId, TaskId};

pub extern "C" fn entry(_ctx: *mut ()) -> ! {
    let runnable = unsafe {
        &mut *(PROCESS_MANAGER.current_task().unwrap().runnable() as *const dyn Runnable
            as *mut dyn Runnable)
    };
    runnable.run()
}

#[test]
fn thread_test() {
    use ::proc::Runnable;
    use alloc::sync::Arc;
    use alloc::vec;
    use atomic::Ordering;
    use core::sync::atomic::AtomicUsize;

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    pub struct TestThread(usize);
    impl Runnable for TestThread {
        fn run(&mut self) -> ! {
            let k = self.0;
            for _ in 0..100 {
                COUNTER.fetch_add(k, Ordering::SeqCst);
            }
            ::syscall::thread_exit()
        }
    }
    // Spawn and distribute worker threads to different cores
    let num_threads = 16;
    let proc = PROCESS_MANAGER.current_proc().unwrap();
    let mut tasks = vec![];
    for i in 0..num_threads {
        let task = proc.clone().spawn_task(box TestThread(i), None);
        tasks.push(task);
    }
    // Wait for all threads to finish
    for task in tasks {
        task.wait_for_completion();
        assert_eq!(Arc::strong_count(&task), 1);
    }
    // Get result
    assert_eq!(
        COUNTER.load(Ordering::SeqCst),
        (0 + num_threads - 1) * num_threads / 2 * 100
    );
}

#[test]
fn smp_test() {
    use crate::arch::{Arch, TargetArch};
    use ::proc::Runnable;
    use alloc::sync::Arc;
    use alloc::vec;
    use atomic::Ordering;
    use core::sync::atomic::AtomicUsize;

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    pub struct Test;
    impl Runnable for Test {
        fn run(&mut self) -> ! {
            let core = TargetArch::current_cpu();
            for _ in 0..100 {
                COUNTER.fetch_add(core, Ordering::SeqCst);
            }
            ::syscall::exit();
        }
    }
    // Spawn and distribute worker processes to different cores
    let num_cpus = TargetArch::num_cpus();
    let mut procs = vec![];
    for i in 0..num_cpus {
        let proc = PROCESS_MANAGER.spawn(box Test, Some(i));
        procs.push(proc);
    }
    // Wait for all processes to finish
    for proc in procs {
        proc.wait_for_completion();
        assert_eq!(Arc::strong_count(&proc), 1);
    }
    // Get result
    assert_eq!(
        COUNTER.load(Ordering::SeqCst),
        (0 + num_cpus - 1) * num_cpus / 2 * 100
    );
}
