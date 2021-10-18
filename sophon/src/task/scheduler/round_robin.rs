use super::*;
use crate::arch::*;
use crate::task::task::Task;
use crate::task::TaskId;
use crate::*;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use core::ops::Deref;
use core::sync::atomic::AtomicUsize;
use crossbeam::queue::SegQueue;
use spin::Mutex;

const UNIT_TIME_SLICE: usize = 1;

#[derive(Debug)]
pub struct State {
    run_state: Atomic<RunState>,
    time_slice_units: AtomicUsize,
}

impl State {
    pub const fn new() -> Self {
        Self {
            run_state: Atomic::new(RunState::Ready),
            time_slice_units: AtomicUsize::new(0),
        }
    }
}

impl AbstractSchedulerState for State {}

impl Deref for State {
    type Target = Atomic<RunState>;
    fn deref(&self) -> &Self::Target {
        &self.run_state
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

pub struct RoundRobinScheduler {
    current_task: [Atomic<Option<TaskId>>; 4],
    tasks: Mutex<BTreeMap<TaskId, Box<Task>>>,
    task_queue: SegQueue<TaskId>,
}

impl AbstractScheduler for RoundRobinScheduler {
    type State = State;

    #[inline]
    fn register_new_task(&self, task: Box<Task>) -> &'static mut Task {
        let _guard = interrupt::uninterruptible();
        let id = task.id;
        let task_ref: &'static mut Task =
            unsafe { &mut *((&task as &Task) as *const Task as usize as *mut Task) };
        self.tasks.lock().insert(id, task);
        if task_ref.scheduler_state::<Self>().load(Ordering::SeqCst) == RunState::Ready {
            debug_assert!(!interrupt::is_enabled());
            self.task_queue.push(id);
        }
        task_ref
    }

    #[inline]
    fn remove_task(&self, id: TaskId) {
        let _task = self.get_task_by_id(id).unwrap();
        self.tasks.lock().remove(&id);
        debug_assert!(!interrupt::is_enabled());
        let _ = self.current_task[0].fetch_update(Ordering::SeqCst, Ordering::SeqCst, |curr| {
            if curr == Some(id) {
                Some(None)
            } else {
                None
            }
        });
    }

    #[inline]
    fn get_task_by_id(&self, id: TaskId) -> Option<&'static Task> {
        let _guard = interrupt::uninterruptible();
        let tasks = self.tasks.lock();
        let task = tasks.get(&id)?;
        let task_ref: &'static mut Task =
            unsafe { &mut *((&task as &Task) as *const Task as usize as *mut Task) };
        Some(task_ref)
    }

    #[inline]
    fn get_current_task_id(&self) -> Option<TaskId> {
        self.current_task[0].load(Ordering::SeqCst)
    }

    #[inline]
    fn get_current_task(&self) -> Option<&'static Task> {
        let _guard = interrupt::uninterruptible();
        self.get_task_by_id(self.get_current_task_id()?)
    }

    #[inline]
    fn mark_task_as_ready(&self, task: &'static Task) {
        assert!(task.scheduler_state::<Self>().load(Ordering::SeqCst) != RunState::Ready);
        task.scheduler_state::<Self>()
            .store(RunState::Ready, Ordering::SeqCst);
        self.task_queue.push(task.id);
    }

    #[inline]
    fn schedule(&self) -> ! {
        interrupt::disable();

        let current_task = self.get_current_task();

        if current_task.is_some()
            && current_task
                .as_ref()
                .unwrap()
                .scheduler_state::<Self>()
                .load(Ordering::SeqCst)
                == RunState::Running
        {
            // Continue with this task
            unsafe {
                current_task.unwrap().context.return_to_user();
            }
        } else {
            // No current task or the current Task is blocked, switch to a new task.

            // Find a schedulable task
            let next_task = self.get_next_schedulable_task();

            debug_assert_eq!(
                next_task.scheduler_state::<Self>().load(Ordering::SeqCst),
                RunState::Ready
            );
            log!(
                "Switch: {:?} -> {:?}",
                current_task.as_ref().map(|t| t.id),
                next_task.id
            );

            // Run next task
            {
                let state = next_task.scheduler_state::<Self>();
                state.run_state.store(RunState::Running, Ordering::SeqCst);
                state
                    .time_slice_units
                    .store(UNIT_TIME_SLICE, Ordering::SeqCst);
            }
            self.set_current_task_id(next_task.id);

            ::core::sync::atomic::fence(Ordering::SeqCst);
            // log!("Schedule return_to_user");
            unsafe {
                next_task.context.return_to_user();
            }
        }
    }

    #[inline]
    fn timer_tick(&self) {
        // log!("Timer TICK");
        debug_assert!(!interrupt::is_enabled());
        let current_task = self.get_current_task().unwrap();

        if current_task
            .scheduler_state::<Self>()
            .time_slice_units
            .load(Ordering::SeqCst)
            == 0
        {
            panic!("time_slice_units is zero");
        }

        {
            let scheduler_state = current_task.scheduler_state::<Self>();
            debug_assert_eq!(
                scheduler_state.run_state.load(Ordering::SeqCst),
                RunState::Running
            );
            let old = scheduler_state
                .time_slice_units
                .fetch_sub(1, Ordering::SeqCst);
            if old == 1 {
                self.enqueue_current_task_as_ready();
                self.schedule();
            } else {
                unsafe { self.get_current_task().unwrap().context.return_to_user() }
            }
        }
    }
}

impl RoundRobinScheduler {
    pub const fn new() -> Self {
        Self {
            current_task: [
                Atomic::new(None),
                Atomic::new(None),
                Atomic::new(None),
                Atomic::new(None),
            ],
            tasks: Mutex::new(BTreeMap::new()),
            task_queue: SegQueue::new(),
        }
    }

    #[inline]
    pub fn set_current_task_id(&self, id: TaskId) {
        self.current_task[0].store(Some(id), Ordering::SeqCst);
    }

    #[inline]
    pub fn enqueue_current_task_as_ready(&self) {
        debug_assert!(!interrupt::is_enabled());
        let task = self.get_current_task().unwrap();
        debug_assert_ne!(
            task.scheduler_state::<Self>().load(Ordering::SeqCst),
            RunState::Ready
        );
        task.scheduler_state::<Self>()
            .store(RunState::Ready, Ordering::SeqCst);
        self.task_queue.push(task.id);
    }

    #[inline]
    fn get_next_schedulable_task(&self) -> &'static Task {
        debug_assert!(!interrupt::is_enabled());
        if let Some(next_runnable_task) = self.task_queue.pop() {
            Task::by_id(next_runnable_task).expect("task not found")
        } else {
            // We should at least have an `idle` task that is runnable
            panic!("No more tasks to run!");
        }
    }
}

unsafe impl Send for RoundRobinScheduler {}
unsafe impl Sync for RoundRobinScheduler {}

pub const fn create() -> Scheduler {
    RoundRobinScheduler::new()
}
