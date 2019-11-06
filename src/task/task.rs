use alloc::boxed::Box;
use alloc::collections::{BTreeMap, LinkedList};
use super::context::*;
use spin::Mutex;
use core::sync::atomic::{AtomicUsize, Ordering};



static TASK_ID_COUNT: AtomicUsize = AtomicUsize::new(1);
static mut CURRENT_TASK: [Option<TaskId>; 4] = [None; 4];

lazy_static! {
    static ref TASKS: Mutex<BTreeMap<TaskId, Box<Task>>> = Mutex::new(BTreeMap::new());
    static ref TASK_QUEUE: Mutex<LinkedList<TaskId>> = Mutex::new(LinkedList::new());
}

const NIL_TASK: Task = Task {
    id: TaskId(0),
    entry: 0 as _,
    state: TaskState::Blocked,
    context: Context::new(),
    stack: [0u8; 4096],
    units: 0,
};

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
    state: TaskState,
    context: Context,
    stack: [u8; 4096],
    units: usize, // Number of unit time slice remaining
}

impl Task {
    pub fn create(entry: extern fn() -> !) -> &'static mut Task {
        // Alloc task struct
        let id = TaskId(TASK_ID_COUNT.fetch_add(1, Ordering::SeqCst));
        let mut task = box Task {
            id,
            entry: entry as _,
            state: TaskState::Ready,
            context: Context::new(),
            stack: [0u8; 4096],
            units: 0,
        };
        let task_ref: &'static mut Task = unsafe { &mut *((&task as &Task) as *const _ as usize as *mut _) };
        // Setup initial context
        task.context.pc = entry as *const fn() as usize;
        task.context.sp = &task.stack as &[u8; 4096] as *const u8 as usize + 4096;
        // Add to tasks set
        TASKS.lock().insert(id, task);
        // Add to task queue
        TASK_QUEUE.lock().push_back(id);
        task_ref
    }

    pub fn by_id(id: TaskId) -> Option<&'static mut Task> {
        let tasks = TASKS.lock();
        let task = tasks.get(&id)?;
        let task_ref: &'static mut Task = unsafe { &mut *((&task as &Task) as *const _ as usize as *mut _) };
        Some(task_ref)
    }

    pub fn current() -> Option<&'static mut Task> {
        Self::by_id(unsafe { CURRENT_TASK[0] }?)
    }

    pub fn schedule() {
        // Find a scheduleable task
        let next_task = {
            if let Some(next_runnable_task) = TASK_QUEUE.lock().pop_front() {
                if let Some(t) = Task::by_id(next_runnable_task) {
                    t
                } else {
                    debug!("Cannot find id {:?}", next_runnable_task);
                    loop {}
                }
            } else {
                // debug!("No more runnable tasks");
                return
            }
        };
        // Switch
        if let Some(curr_task) = Task::current() {
            curr_task.switch_to(next_task)
        } else {
            NIL_TASK.switch_to(next_task)
        }
    }

    pub fn timer_tick() {
        let current = match Task::current() {
            Some(t) => t,
            _ => return,
        };
        if current.units == 0 {
            return
        }
        current.units -= 1;
        if current.units > 0 {
            return
        }
        // Run out of time slice, reschedule
        Task::schedule();
    }

    fn switch_to(&mut self, to_task: &'static mut Task) {
        // debug!("Switch: {:?} -> {:?}", self.id, to_task.id);
        if self.id == to_task.id {
            return // Do nothing
        }
        // Add this task to ready queue
        if self.id != NIL_TASK.id {
            TASK_QUEUE.lock().push_back(self.id);
        }
        // Run next task
        to_task.state = TaskState::Running;
        unsafe {
            CURRENT_TASK[0] = Some(to_task.id);
        }
        to_task.units = 100;
        // Do the actual context switch
        crate::interrupt::enable();
        unsafe {
            self.context.switch_to(&to_task.context);
        }
        crate::interrupt::disable();
    }

    #[inline]
    pub fn id(&self) -> TaskId {
        self.id
    }

    #[inline]
    pub fn state(&self) -> TaskState {
        self.state
    }
}

unsafe impl Send for Task {}
unsafe impl Sync for Task {}
