pub mod proc;
pub mod runnable;
pub mod syscall;
pub mod task;

pub use self::proc::*;
pub use ::proc::{ProcId, TaskId};
pub use task::*;

#[test]
fn smp_test() {
    use crate::arch::{Arch, TargetArch};
    use crate::task::runnable::Runnable;
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
        let proc = Proc::spawn(box Test, Some(i));
        procs.push(proc);
    }
    // Wait for all processes to finish
    for proc in procs {
        let mut live = proc.live.lock();
        while *live {
            live = proc.live.wait(live);
        }
    }
    // Get result
    assert_eq!(
        COUNTER.load(Ordering::SeqCst),
        (0 + num_cpus - 1) * num_cpus / 2 * 100
    );
}

#[test]
fn thread_test() {
    use crate::arch::{Arch, TargetArch};
    use crate::task::runnable::Runnable;
    use alloc::vec;
    use atomic::Ordering;
    use core::sync::atomic::AtomicUsize;

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    pub struct TestThread;
    impl Runnable for TestThread {
        fn run(&mut self) -> ! {
            let core = TargetArch::current_cpu();
            for _ in 0..100 {
                COUNTER.fetch_add(core, Ordering::SeqCst);
            }
            ::syscall::thread_exit()
        }
    }
    // Spawn and distribute worker threads to different cores
    let num_cpus = TargetArch::num_cpus();
    let proc = Proc::current();
    let mut tasks = vec![];
    for i in 0..num_cpus {
        let task = proc.spawn_kernel_task(box TestThread, Some(i));
        tasks.push(task);
    }
    // Wait for all threads to finish
    for task in tasks {
        let mut live = task.live.lock();
        while *live {
            live = task.live.wait(live);
        }
    }
    // Get result
    assert_eq!(
        COUNTER.load(Ordering::SeqCst),
        (0 + num_cpus - 1) * num_cpus / 2 * 100
    );
}
