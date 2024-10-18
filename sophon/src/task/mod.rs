pub mod proc;
pub mod runnables;
pub mod syscall;

pub use self::proc::*;
use crate::modules::PROCESS_MANAGER;
use ::proc::Runnable;
pub use ::proc::{ProcId, TaskId};

#[allow(invalid_reference_casting)]
pub extern "C" fn entry(_ctx: *mut ()) -> ! {
    let runnable = unsafe {
        let task = PROCESS_MANAGER.current_task();
        let runnable_ptr = task.as_ref().unwrap().runnable() as *const dyn Runnable;
        &mut *(runnable_ptr as *mut dyn Runnable)
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
    use alloc::boxed::Box;
    for i in 0..num_threads {
        let task = proc.clone().spawn_task(Box::new(TestThread(i)));
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
