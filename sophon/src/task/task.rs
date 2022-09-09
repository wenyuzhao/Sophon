use super::TaskId;
use crate::arch::Arch;
use crate::arch::ArchContext;
use crate::arch::TargetArch;
use crate::modules::SCHEDULER;
use crate::*;
use alloc::boxed::Box;
use alloc::sync::{Arc, Weak};
use core::any::Any;
use core::sync::atomic::{AtomicUsize, Ordering};
use proc::Proc;
use proc::Runnable;
use spin::Lazy;
use sync::Monitor;

// static TASK_ID_COUNT: AtomicUsize = AtomicUsize::new(0);

// pub struct Task {
//     pub id: TaskId,
//     pub context: <TargetArch as Arch>::Context,
//     proc: Weak<dyn Proc>,
//     pub live: Lazy<Monitor<bool>>,
//     pub sched: Box<dyn Any>,
//     runnable: Box<dyn Runnable>,
// }

// impl Task {
//     pub(super) fn create(proc: Arc<dyn Proc>, runnable: Box<dyn Runnable>) -> Arc<Self> {
//         let id = TaskId(TASK_ID_COUNT.fetch_add(1, Ordering::SeqCst));
//         Arc::new(Task {
//             id,
//             context: <TargetArch as Arch>::Context::new(entry as _, 0 as _),
//             proc: Arc::downgrade(&proc),
//             live: Lazy::new(|| Monitor::new(true)),
//             sched: SCHEDULER.new_state(),
//             runnable,
//         })
//     }

//     pub fn by_id(id: TaskId) -> Option<Arc<Self>> {
//         SCHEDULER.get_task_by_id(id)
//     }

//     pub fn current() -> Arc<Self> {
//         SCHEDULER.get_current_task().unwrap()
//     }

//     pub fn current_opt() -> Option<Arc<Self>> {
//         SCHEDULER.get_current_task()
//     }

//     pub fn get_context_ptr<C: ArchContext>(&self) -> *const C {
//         let ptr = &self.context as *const _;
//         ptr as *const C
//     }

//     pub fn get_context<C: ArchContext>(&self) -> &C {
//         let ptr = &self.context as *const _;
//         unsafe { &*(ptr as *const C) }
//     }

//     pub fn proc(&self) -> Arc<dyn Proc> {
//         self.proc.upgrade().unwrap()
//     }

//     pub fn exit(&self) {
//         assert!(!interrupt::is_enabled());
//         assert_eq!(self.id, Task::current().id);
//         // Mark as dead
//         {
//             let mut live = self.live.lock();
//             *live = false;
//             self.live.notify_all()
//         }
//         // Remove from scheduler
//         SCHEDULER.remove_task(Task::current().id);
//         self.proc
//             .upgrade()
//             .unwrap()
//             .tasks()
//             .lock()
//             .drain_filter(|t| *t == self.id);
//     }
// }

// unsafe impl Send for Task {}
// unsafe impl Sync for Task {}

// impl PartialEq for Task {
//     fn eq(&self, other: &Self) -> bool {
//         self.id == other.id
//     }
// }

// impl Eq for Task {}

extern "C" fn entry(_ctx: *mut ()) -> ! {
    let runnable = unsafe {
        &mut *(Task::current().runnable.as_ref() as *const dyn Runnable as *mut dyn Runnable)
    };
    runnable.run()
}
