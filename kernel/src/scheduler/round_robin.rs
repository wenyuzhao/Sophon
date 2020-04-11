use super::*;
use spin::Mutex;
use alloc::collections::{BTreeMap, LinkedList};
use alloc::boxed::Box;
use core::cell::UnsafeCell;
use crate::arch::*;
use crate::*;
use core::ops::{Deref, DerefMut};



const UNIT_TIME_SLICE: usize = 1;

#[derive(Debug, Clone)]
pub struct State {
    run_state: RunState,
    time_slice_units: usize,
}

impl State {
    pub const fn new() -> Self {
        Self {
            run_state: RunState::Ready,
            time_slice_units: 0
        }
    }
}

impl SchedulerState for State {}

impl Deref for State {
    type Target = RunState;
    fn deref(&self) -> &Self::Target {
        &self.run_state
    }
}

impl DerefMut for State {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.run_state
    }
}

impl Default for State {
    fn default() -> Self { Self::new() }
}

pub struct RoundRobinScheduler<K: AbstractKernel<Scheduler=Self>> {
    current_task: UnsafeCell<[Option<TaskId>; 4]>,
    tasks: Mutex<BTreeMap<TaskId, Box<Task<K>>>>,
    task_queue: Mutex<LinkedList<TaskId>>,
}

impl <K: AbstractKernel<Scheduler=Self>> AbstractScheduler for RoundRobinScheduler<K> {
    type State = State;
    type Kernel = K;

    fn new() -> Self {
        Self {
            current_task: UnsafeCell::new([None; 4]),
            tasks: Mutex::new(BTreeMap::new()),
            task_queue: Mutex::new(LinkedList::new()),
        }
    }

    fn register_new_task(&self, task: Box<Task<K>>) -> &'static mut Task<K> {
        Self::uninterruptable(|| {
            let id = task.id();
            let task_ref: &'static mut Task<K> = unsafe { &mut *((&task as &Task<K>) as *const Task<K> as usize as *mut Task<K>) };
            self.tasks.lock().insert(id, task);
            if task_ref.scheduler_state().borrow().run_state == RunState::Ready {
                debug_assert!(!<K::Arch as AbstractArch>::Interrupt::is_enabled());
                self.task_queue.lock().push_back(id);
            }
            task_ref
        })
    }

    fn remove_task(&self, id: TaskId) {
        let _task = self.get_task_by_id(id).unwrap();
        self.tasks.lock().remove(&id);
        debug_assert!(!<K::Arch as AbstractArch>::Interrupt::is_enabled());
        debug_assert!(!self.task_queue.lock().contains(&id));
        let current_task_table = unsafe { &mut *self.current_task.get() };
        current_task_table[0] = None;
        unimplemented!()
    }

    fn get_task_by_id(&self, id: TaskId) -> Option<&'static mut Task<K>> {
        Self::uninterruptable(|| {
            let tasks = self.tasks.lock();
            let task = tasks.get(&id)?;
            let task_ref: &'static mut Task<K> = unsafe { &mut *((&task as &Task<K>) as *const Task<K> as usize as *mut Task<K>) };
            Some(task_ref)
        })
    }

    fn get_current_task_id(&self) -> Option<TaskId> {
        let current_task_table = unsafe { &*self.current_task.get() };
        current_task_table[0]
    }

    fn get_current_task(&self) -> Option<&'static mut Task<K>> {
        Self::uninterruptable(|| {
            self.get_task_by_id(self.get_current_task_id()?)
        })
    }

    fn mark_task_as_ready(&self, task: &'static mut Task<K>) {
        assert!(task.scheduler_state().borrow().run_state != RunState::Ready);
        **task.scheduler_state().borrow_mut() = RunState::Ready;
        self.task_queue.lock().push_back(task.id());
    }

    fn schedule(&self) -> ! {
        <K::Arch as AbstractArch>::Interrupt::disable();

        let current_task = self.get_current_task();

        if current_task.is_some() && current_task.as_ref().unwrap().scheduler_state().borrow().run_state == RunState::Running {
            // Continue with this task
            unsafe { current_task.unwrap().context.return_to_user(); }
        } else {
            // Current Task is blocked, switch to a new task

            // Find a scheduleable task
            let next_task = self.get_next_schedulable_task();
            
            debug_assert!({
                let state = next_task.scheduler_state().borrow_mut();
                state.run_state == RunState::Ready
            });
            debug!(K: "Switch: {:?} -> {:?}", current_task.as_ref().map(|t| t.id()), next_task.id());
            
            // Run next task
            {
                let mut state = next_task.scheduler_state().borrow_mut();
                state.run_state = RunState::Running;
                state.time_slice_units = UNIT_TIME_SLICE;
            }
            self.set_current_task_id(next_task.id());
    
            ::core::sync::atomic::fence(::core::sync::atomic::Ordering::SeqCst);
            debug!(K: "Schedule return_to_user");
            unsafe { next_task.context.return_to_user(); }
        }
    }

    fn timer_tick(&self) {
        debug_assert!(!<K::Arch as AbstractArch>::Interrupt::is_enabled());
        let current_task = self.get_current_task().unwrap();

        if current_task.scheduler_state().borrow().time_slice_units == 0 {
            panic!("time_slice_units is zero");
        }

        {
            let mut scheduler_state = current_task.scheduler_state().borrow_mut();
            debug_assert!(scheduler_state.run_state == RunState::Running, "Invalid state {:?} for {:?}", scheduler_state.run_state, current_task.id());
            scheduler_state.time_slice_units -= 1;
            if scheduler_state.time_slice_units == 0 {
                debug!(K: "Schedule");
                scheduler_state.time_slice_units = UNIT_TIME_SLICE;
                ::core::mem::drop(scheduler_state);
                self.enqueue_current_task_as_ready();
                self.schedule();
            } else {
                ::core::mem::drop(scheduler_state);
                unsafe { self.get_current_task().unwrap().context.return_to_user() }
            }
        }
    }
}

impl <K: AbstractKernel<Scheduler=Self>> RoundRobinScheduler<K> {

    pub fn set_current_task_id(&self, id: TaskId) {
        let current_task_table = unsafe { &mut *self.current_task.get() };
        current_task_table[0] = Some(id);
    }

    // 

    pub fn enqueue_current_task_as_ready(&self) {
        debug_assert!(!<K::Arch as AbstractArch>::Interrupt::is_enabled());
        let task = self.get_current_task().unwrap();
        assert!(task.scheduler_state().borrow().run_state != RunState::Ready);
        task.scheduler_state().borrow_mut().run_state = RunState::Ready;
        self.task_queue.lock().push_back(task.id());
    }

    fn get_next_schedulable_task(&self) -> &'static mut Task<K> {
        debug_assert!(!<K::Arch as AbstractArch>::Interrupt::is_enabled());
        if let Some(next_runnable_task) = self.task_queue.lock().pop_front() {
            Task::by_id(next_runnable_task).expect("task not found")
        } else {
            // We should at least have an `idle` task that is runnable
            panic!("No more tasks to run!");
        }
    }
}

unsafe impl <K: AbstractKernel<Scheduler=Self>> Send for RoundRobinScheduler<K> {}
unsafe impl <K: AbstractKernel<Scheduler=Self>> Sync for RoundRobinScheduler<K> {}
