#![feature(format_args_nl)]
#![feature(default_alloc_error_handler)]
#![feature(box_syntax)]
#![feature(generic_associated_types)]
#![feature(downcast_unchecked)]
#![no_std]

#[allow(unused)]
#[macro_use]
extern crate log;
extern crate alloc;

use alloc::{boxed::Box, vec::Vec};
use atomic::{Atomic, Ordering};
use core::{any::Any, sync::atomic::AtomicUsize};
use crossbeam::queue::SegQueue;
use kernel_module::{kernel_module, KernelModule, ProcessorLocalStorage, SERVICE};
use proc::TaskId;
use sched::{RunState, Scheduler};
use spin::Lazy;

const UNIT_TIME_SLICE: usize = 1;

#[derive(Debug)]
pub struct State {
    run_state: Atomic<RunState>,
    time_slice_units: AtomicUsize,
    affinity: AtomicUsize,
}

impl State {
    pub const fn new() -> Self {
        Self {
            run_state: Atomic::new(RunState::Ready),
            time_slice_units: AtomicUsize::new(0),
            affinity: AtomicUsize::new(usize::MAX),
        }
    }
}

#[kernel_module]
pub static mut SCHEDULER: RoundRobinScheduler = RoundRobinScheduler::new();

pub struct RoundRobinScheduler {
    current_task: Vec<Atomic<Option<TaskId>>>,
    per_core_task_queue: Lazy<ProcessorLocalStorage<SegQueue<TaskId>>>,
}

impl RoundRobinScheduler {
    const fn new() -> Self {
        Self {
            current_task: Vec::new(),
            per_core_task_queue: Lazy::new(|| ProcessorLocalStorage::new()),
        }
    }

    #[inline]
    fn set_current_task_id(&self, id: TaskId) {
        self.current_task[SERVICE.current_core()].store(Some(id), Ordering::SeqCst);
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
    fn get_state(&self, task: TaskId) -> &State {
        let task = SERVICE.process_manager().get_task_by_id(task).unwrap();
        debug_assert!(task.sched().is::<State>());
        let state = task.sched() as *const dyn Any;
        unsafe { (*(state as *const dyn Any)).downcast_ref_unchecked::<State>() }
    }

    #[inline]
    pub fn enqueue_current_task_as_ready(&self) {
        debug_assert!(!interrupt::is_enabled());
        let task = self.get_current_task_id().unwrap();
        let state = self.get_state(task);
        debug_assert_ne!(state.run_state.load(Ordering::SeqCst), RunState::Ready);
        state.run_state.store(RunState::Ready, Ordering::SeqCst);
        let affinity = state.affinity.load(Ordering::SeqCst);
        self.per_core_task_queue.get(affinity).push(task);
    }
}

impl Scheduler for RoundRobinScheduler {
    fn new_state(&self) -> Box<dyn Any> {
        Box::new(State::new())
    }

    fn get_current_task_id(&self) -> Option<TaskId> {
        self.current_task[SERVICE.current_core()].load(Ordering::SeqCst)
    }

    fn register_new_task(&self, task: TaskId, affinity: Option<usize>) {
        let _guard = interrupt::uninterruptible();
        let affinity = if let Some(affinity) = affinity {
            affinity
        } else {
            let mut size = self.per_core_task_queue.get(0).len();
            let mut affinity = 0;
            for i in 1..SERVICE.num_cores() {
                let s = self.per_core_task_queue.get(i).len();
                if s < size {
                    affinity = i;
                    size = s;
                }
            }
            affinity
        };
        let state = self.get_state(task);
        state.affinity.store(affinity, Ordering::SeqCst);
        if state.run_state.load(Ordering::SeqCst) == RunState::Ready {
            debug_assert!(!interrupt::is_enabled());
            self.per_core_task_queue.get(affinity).push(task);
        }
    }

    fn remove_task(&self, task: TaskId) {
        debug_assert!(!interrupt::is_enabled());
        let _ = self.current_task[SERVICE.current_core()].fetch_update(
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
    }

    fn sleep(&self) {
        let _guard = interrupt::uninterruptible();
        let task = self.get_current_task_id().unwrap();
        let state = self.get_state(task);
        assert_eq!(state.run_state.load(Ordering::SeqCst), RunState::Running);
        state.run_state.store(RunState::Sleeping, Ordering::SeqCst);
        self.schedule();
    }

    fn wake_up(&self, task: TaskId) {
        let _guard = interrupt::uninterruptible();
        let state = self.get_state(task);
        let old = state
            .run_state
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |old| {
                if old == RunState::Sleeping {
                    Some(RunState::Ready)
                } else {
                    None
                }
            });
        if old == Ok(RunState::Sleeping) {
            let affinity = state.affinity.load(Ordering::SeqCst);
            self.per_core_task_queue.get(affinity).push(task);
        }
    }

    fn schedule(&self) -> ! {
        interrupt::disable();

        let current_task = self.get_current_task_id();
        let state = current_task.map(|t| self.get_state(t));

        if current_task.is_some()
            && state.unwrap().run_state.load(Ordering::SeqCst) == RunState::Running
        {
            // Continue with this task
            unsafe { SERVICE.return_to_user(current_task.unwrap()) }
        } else {
            // No current task or the current Task is blocked, switch to a new task.

            // Find a schedulable task
            let next_task = self.get_next_schedulable_task();

            // if current_task.as_ref().map(|t| t.id) != Some(next_task.id) {
            //     static SYNC: Mutex<()> = Mutex::new(());
            //     let _guard = SYNC.lock();
            //     log!(
            //         "Switch(#{}): {:?} -> {:?}",
            //         TargetArch::current_cpu(),
            //         current_task.as_ref().map(|t| t.id),
            //         next_task.id
            //     );
            // }
            // Run next task
            {
                let state = self.get_state(next_task);
                debug_assert_eq!(state.run_state.load(Ordering::SeqCst), RunState::Ready);
                state.run_state.store(RunState::Running, Ordering::SeqCst);
                state
                    .time_slice_units
                    .store(UNIT_TIME_SLICE, Ordering::SeqCst);
            }
            self.set_current_task_id(next_task);
            atomic::fence(Ordering::SeqCst);
            // Return to user
            unsafe { SERVICE.return_to_user(next_task) }
        }
    }

    fn timer_tick(&self) -> ! {
        debug_assert!(!interrupt::is_enabled());
        let current_task = self.get_current_task_id().unwrap();
        let state = self.get_state(current_task);

        if state.time_slice_units.load(Ordering::SeqCst) == 0 {
            panic!("time_slice_units is zero");
        }

        {
            debug_assert_eq!(state.run_state.load(Ordering::SeqCst), RunState::Running);
            let old = state.time_slice_units.fetch_sub(1, Ordering::SeqCst);
            if old == 1 {
                self.enqueue_current_task_as_ready();
                self.schedule();
            } else {
                unsafe { SERVICE.return_to_user(current_task) }
            }
        }
    }
}

impl KernelModule for RoundRobinScheduler {
    fn init(&'static mut self) -> anyhow::Result<()> {
        self.current_task
            .resize_with(SERVICE.num_cores(), || Atomic::new(None));
        SERVICE.set_scheduler(self);
        Ok(())
    }
}
