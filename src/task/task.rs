use alloc::boxed::Box;
use alloc::collections::{BTreeMap, LinkedList};
use super::context::*;
use spin::Mutex;
use core::sync::atomic::{AtomicUsize, Ordering};
use alloc::vec::Vec;
use super::scheduler::*;
use core::cell::{RefMut, RefCell};



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
    entry: *const extern fn() -> !,
    scheduler_state: RefCell<SchedulerState>,
    context: Context,
    stack: Vec<u8>,
}

impl Task {
    #[inline]
    pub fn id(&self) -> TaskId {
        self.id
    }

    #[inline]
    pub fn scheduler_state(&self) -> &RefCell<SchedulerState> {
        // if { self.scheduler_state.try_borrow_mut().is_err() } {
        //     debug!("Value borrowed twice!");
        //     loop {}
        // }
        &self.scheduler_state
    }
}

impl Task {
    pub fn create(entry: extern fn() -> !) -> &'static mut Task {
        // Alloc task struct
        let id = TaskId(TASK_ID_COUNT.fetch_add(1, Ordering::SeqCst));
        let stack = vec![0u8; 4096 * 2];
        let task = box Task {
            id,
            entry: entry as _,
            context: Context::new(entry as _, stack.as_ptr()),
            stack,
            scheduler_state: RefCell::new(SchedulerState::new()),
        };
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
