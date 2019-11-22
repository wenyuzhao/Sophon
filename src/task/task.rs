use alloc::boxed::Box;
use alloc::collections::{BTreeMap, LinkedList};
use super::context::*;
use spin::Mutex;
use core::sync::atomic::{AtomicUsize, Ordering};
use alloc::vec::Vec;
use super::scheduler::*;
use core::cell::{RefMut, RefCell};
use crate::mm::*;
use crate::exception::ExceptionFrame;
use crate::mm::heap_constants::*;

use core::iter::Step;



static TASK_ID_COUNT: AtomicUsize = AtomicUsize::new(1);


#[derive(Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Clone, Copy)]
pub enum TaskState {
    Ready,
    Running,
    Blocked,
}

#[derive(Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Clone, Copy)]
pub struct TaskId(usize);



pub struct Task {
    id: TaskId,
    scheduler_state: RefCell<SchedulerState>,
    context: Context,
}

impl Task {
    #[inline]
    pub fn id(&self) -> TaskId {
        self.id
    }

    #[inline]
    pub fn scheduler_state(&self) -> &RefCell<SchedulerState> {
        &self.scheduler_state
    }

    /// Fork a new task.
    /// This will duplicate the virtual memory
    pub fn fork(&self, parent_sp: usize) -> &'static mut Task {
        let id = TaskId(TASK_ID_COUNT.fetch_add(1, Ordering::SeqCst));
        // Allocate task struct
        let task = box Task {
            id,
            context: self.context.fork(parent_sp, 0),
            scheduler_state: RefCell::new(SchedulerState::new()),
        };
        GLOBAL_TASK_SCHEDULER.register_new_task(task)
    }
}

impl Task {
    /// Create a init task with empty p4 table
    pub fn create_kernel_task(entry: extern fn() -> !) -> &'static mut Task {
        // Assign an id
        let id = TaskId(TASK_ID_COUNT.fetch_add(1, Ordering::SeqCst));
        // Alloc task struct
        let mut task = box Task {
            id,
            context: Context::new(entry as _),
            scheduler_state: RefCell::new(SchedulerState::new()),
        };
        // Add this task to the schedular
        GLOBAL_TASK_SCHEDULER.register_new_task(task)
    }

    pub fn by_id(id: TaskId) -> Option<&'static mut Task> {
        GLOBAL_TASK_SCHEDULER.get_task_by_id(id)
    }

    pub fn current() -> Option<&'static mut Task> {
        GLOBAL_TASK_SCHEDULER.get_current_task()
    }

    pub fn switch(from_task: Option<&'static mut Task>, to_task: &'static mut Task) {
        debug_assert!(from_task != Some(to_task), "{:?} {:?}", from_task.as_ref().map(|t| t.id), to_task.id);
        crate::interrupt::enable();
        unsafe {
            if let Some(from_task) = from_task {
                from_task.context.switch_to(&to_task.context);
            } else {
                let mut temp_ctx = Context::empty();
                temp_ctx.switch_to(&to_task.context);
            }
        }
        crate::interrupt::disable();
    }
}

unsafe impl Send for Task {}
unsafe impl Sync for Task {}

impl PartialEq for Task {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Task {}
