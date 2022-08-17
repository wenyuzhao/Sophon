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
    use mutex::AbstractMonitor;

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
        while !proc.dead.load(Ordering::SeqCst) {
            proc.monitor.wait();
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
    use crate::scheduler::monitor::SysMonitor;
    use crate::task::runnable::Runnable;
    use alloc::sync::Arc;
    use atomic::Ordering;
    use core::sync::atomic::AtomicUsize;
    use mutex::AbstractMonitor;
    use spin::Lazy;

    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    static COMPLETED_THREADS: AtomicUsize = AtomicUsize::new(0);
    static MONITOR: Lazy<Arc<SysMonitor>> = Lazy::new(|| SysMonitor::new());

    pub struct TestThread;
    impl Runnable for TestThread {
        fn run(&mut self) -> ! {
            let core = TargetArch::current_cpu();
            for _ in 0..100 {
                COUNTER.fetch_add(core, Ordering::SeqCst);
            }
            COMPLETED_THREADS.fetch_add(1, Ordering::SeqCst);
            MONITOR.notify();
            ::syscall::thread_exit()
        }
    }
    // Spawn and distribute worker threads to different cores
    let num_cpus = TargetArch::num_cpus();
    let proc = Proc::current();
    for i in 0..num_cpus {
        let _task = proc.spawn_kernel_task(box TestThread, Some(i));
    }
    // Wait for all threads to finish
    while COMPLETED_THREADS.load(Ordering::SeqCst) != num_cpus {
        MONITOR.wait();
    }
    // Get result
    assert_eq!(
        COUNTER.load(Ordering::SeqCst),
        (0 + num_cpus - 1) * num_cpus / 2 * 100
    );
}
