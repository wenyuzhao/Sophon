use core::sync::atomic::Ordering;

use crate::arch::{Arch, ArchContext, TargetArch};
use alloc::{boxed::Box, collections::btree_map::BTreeMap, sync::Arc, vec::Vec};
use atomic::Atomic;
use crossbeam::queue::SegQueue;
use klib::task::{RunState, Task, TaskId};
use spin::{Lazy, Mutex};

use crate::utils::pls::ProcessorLocalStorage;

pub struct Scheduler {
    current_task: Lazy<Vec<Atomic<Option<TaskId>>>>,
    per_core_task_queue: Lazy<ProcessorLocalStorage<SegQueue<TaskId>>>,
    tasks: Mutex<BTreeMap<TaskId, Arc<Task>>>,
}

unsafe impl Sync for Scheduler {}

pub static SCHEDULER: Scheduler = Scheduler::new();

const UNIT_TIME_SLICE: usize = 1;

impl Scheduler {
    const fn new() -> Self {
        Self {
            current_task: Lazy::new(|| {
                (0..ProcessorLocalStorage::<usize>::num_cores())
                    .map(|_| Atomic::new(None))
                    .collect()
            }),
            per_core_task_queue: Lazy::new(|| ProcessorLocalStorage::new()),
            tasks: Mutex::new(BTreeMap::new()),
        }
    }

    fn current_core() -> usize {
        ProcessorLocalStorage::<usize>::current_core()
    }

    pub fn get_current_task(&self) -> Option<Arc<Task>> {
        self.get_current_task_id().map(|id| self.get_task_by_id(id))
    }

    pub fn get_current_task_id(&self) -> Option<TaskId> {
        self.current_task[Self::current_core()].load(Ordering::Relaxed)
    }

    #[inline]
    fn set_current_task_id(&self, id: TaskId) {
        self.current_task[Self::current_core()].store(Some(id), Ordering::Relaxed);
    }

    #[inline]
    fn get_next_schedulable_task(&self) -> TaskId {
        debug_assert!(!interrupt::is_enabled());
        if let Some(next_runnable_task) = self.per_core_task_queue.pop() {
            return next_runnable_task;
        } else {
            // We should at least have an `idle` task that is runnable
            panic!("No more tasks to run!");
        }
    }

    #[inline]
    fn get_task_by_id(&self, task: TaskId) -> Arc<Task> {
        self.tasks.lock().get(&task).unwrap().clone()
    }

    #[inline]
    pub fn enqueue_current_task_as_ready(&self) {
        debug_assert!(!interrupt::is_enabled());
        let tid = self.get_current_task_id().unwrap();
        let task = self.get_task_by_id(tid);
        debug_assert_ne!(task.state.load(Ordering::SeqCst), RunState::Ready);
        task.state.store(RunState::Ready, Ordering::SeqCst);
        self.per_core_task_queue.get(0).push(tid);
    }

    pub fn register_new_task(&self, task: Arc<Task>) {
        let _guard = interrupt::uninterruptible();
        self.tasks.lock().insert(task.id, task.clone());
        if task.state.load(Ordering::SeqCst) == RunState::Ready {
            debug_assert!(!interrupt::is_enabled());
            self.per_core_task_queue.get(0).push(task.id);
        }
    }

    pub fn remove_task(&self, task: TaskId) {
        debug_assert!(!interrupt::is_enabled());
        let _ = self.current_task[Self::current_core()].fetch_update(
            Ordering::SeqCst,
            Ordering::SeqCst,
            |curr| {
                if curr == Some(task) {
                    Some(None)
                } else {
                    None
                }
            },
        );
        self.tasks.lock().remove(&task).unwrap();
    }

    pub(super) fn create_task_context(&self) -> Box<<TargetArch as Arch>::Context> {
        Box::new(<TargetArch as Arch>::Context::new(
            crate::task::entry as _,
            0 as _,
        ))
    }

    pub fn sleep(&self) {
        let _guard = interrupt::uninterruptible();
        let tid = self.get_current_task_id().unwrap();
        let task = self.get_task_by_id(tid);
        assert_eq!(task.state.load(Ordering::SeqCst), RunState::Running);
        task.state.store(RunState::Blocked, Ordering::SeqCst);
        self.schedule();
    }

    #[allow(unused)]
    fn wake_up(&self, tid: TaskId) {
        let _guard = interrupt::uninterruptible();

        let task = self.get_task_by_id(tid);
        let old = task
            .state
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |old| {
                if old == RunState::Blocked {
                    Some(RunState::Ready)
                } else {
                    None
                }
            });
        if old == Ok(RunState::Blocked) {
            self.per_core_task_queue.get(0).push(tid);
        }
    }

    pub fn schedule(&self) -> ! {
        interrupt::disable();

        let tid = self.get_current_task_id();
        let task = tid.map(|t| self.get_task_by_id(t));

        if tid.is_some() && task.as_ref().unwrap().state.load(Ordering::SeqCst) == RunState::Running
        {
            // Continue with this task
            unsafe { self.return_to_user(task.unwrap()) }
        } else {
            // No current task or the current Task is blocked, switch to a new task.

            // Find a schedulable task
            let next_task_id = self.get_next_schedulable_task();
            let next_task = self.get_task_by_id(next_task_id);

            if task.as_ref().map(|t| t.id) != Some(next_task.id) {
                //     static SYNC: Mutex<()> = Mutex::new(());
                //     let _guard = SYNC.lock();
                trace!(
                    "Switch: {:?} -> {:?}",
                    task.as_ref().map(|t| t.id),
                    next_task.id
                );
            }
            // Run next task
            {
                debug_assert_eq!(next_task.state.load(Ordering::SeqCst), RunState::Ready);
                next_task.state.store(RunState::Running, Ordering::SeqCst);
                next_task.ticks.store(UNIT_TIME_SLICE, Ordering::SeqCst);
            }
            self.set_current_task_id(next_task_id);
            atomic::fence(Ordering::SeqCst);
            // Return to user
            unsafe { self.return_to_user(next_task) }
        }
    }

    unsafe fn return_to_user(&self, task: Arc<Task>) -> ! {
        // Note: `task` must be dropped before calling `return_to_user`.
        let context_ptr = {
            task.context
                .downcast_ref_unchecked::<<TargetArch as Arch>::Context>()
                as *const <TargetArch as Arch>::Context
        };
        drop(task);
        (*context_ptr).return_to_user()
    }

    pub fn timer_tick(&self) -> ! {
        debug_assert!(!interrupt::is_enabled());
        let tid = self.get_current_task_id().unwrap();
        let task = self.get_task_by_id(tid);

        if task.ticks.load(Ordering::SeqCst) == 0 {
            panic!("time_slice_units is zero");
        }

        {
            debug_assert_eq!(task.state.load(Ordering::SeqCst), RunState::Running);
            let old = task.ticks.fetch_sub(1, Ordering::SeqCst);
            if old == 1 {
                self.enqueue_current_task_as_ready();
                self.schedule();
            } else {
                unsafe { self.return_to_user(task) }
            }
        }
    }
}
