use super::*;
use spin::Mutex;
use alloc::collections::{BTreeMap, LinkedList};
use alloc::boxed::Box;
use core::cell::UnsafeCell;
use crate::arch::*;


lazy_static! {
    pub static ref GLOBAL_TASK_SCHEDULER: Scheduler = Scheduler::new();
}
/**
 *                        ___________
 *                       |           |
 *                       v           |
 * [CreateProcess] --> Ready ---> Running
 *                       ^           |
 *                       |           v
 *                       |___ Sending/Receiving
 * 
 */
#[derive(Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Clone, Copy)]
enum RunState {
    Ready,
    Running,
    Sending,
    Receiving,
}

#[derive(Debug, Clone)]
pub struct SchedulerState {
    run_state: RunState,
    time_slice_units: usize,
}

impl SchedulerState {
    pub const fn new() -> Self {
        Self {
            run_state: RunState::Ready,
            time_slice_units: 0
        }
    }
}

pub struct Scheduler {
    current_task: UnsafeCell<[Option<TaskId>; 4]>,
    tasks: Mutex<BTreeMap<TaskId, Box<Task>>>,
    task_queue: Mutex<LinkedList<TaskId>>,
}

impl Scheduler {
    fn new() -> Self {
        Self {
            current_task: UnsafeCell::new([None; 4]),
            tasks: Mutex::new(BTreeMap::new()),
            task_queue: Mutex::new(LinkedList::new()),
        }
    }

    pub fn register_new_task(&self, mut task: Box<Task>) -> &'static mut Task {
        uninterruptable! {{
            let id = task.id();
            let task_ref: &'static mut Task = unsafe { &mut *((&task as &Task) as *const Task as usize as *mut Task) };
            self.tasks.lock().insert(id, task);
            if task_ref.scheduler_state().borrow().run_state == RunState::Ready {
                debug_assert!(!Target::Interrupt::is_enabled());
                self.task_queue.lock().push_back(id);
            }
            task_ref
        }}
    }

    pub fn remove_task(&self, id: TaskId) {
        let task = self.get_task_by_id(id).unwrap();
        self.tasks.lock().remove(&id);
        debug_assert!(!Target::Interrupt::is_enabled());
        debug_assert!(!self.task_queue.lock().contains(&id));
        let current_task_table = unsafe { &mut *self.current_task.get() };
        current_task_table[0] = None;
        unimplemented!()
        // self.schedule()
    }

    pub fn get_task_by_id(&self, id: TaskId) -> Option<&'static mut Task> {
        uninterruptable! {{
            let tasks = self.tasks.lock();
            let task = tasks.get(&id)?;
            let task_ref: &'static mut Task = unsafe { &mut *((&task as &Task) as *const Task as usize as *mut Task) };
            Some(task_ref)
        }}
    }

    pub fn get_current_task_id(&self) -> Option<TaskId> {
        let current_task_table = unsafe { &*self.current_task.get() };
        current_task_table[0]
    }

    pub fn set_current_task_id(&self, id: TaskId) {
        let current_task_table = unsafe { &mut *self.current_task.get() };
        current_task_table[0] = Some(id);
    }
    
    pub fn get_current_task(&self) -> Option<&'static mut Task> {
        uninterruptable! {{
            self.get_task_by_id(self.get_current_task_id()?)
        }}
    }

    pub fn unblock_sending_task(&self, id: TaskId) {
        uninterruptable! {{
            let task = self.get_task_by_id(id).unwrap();
            assert!(task.scheduler_state().borrow().run_state == RunState::Sending);
            // Add this task to ready queue
            task.scheduler_state().borrow_mut().run_state = RunState::Ready;
            debug_assert!(!Target::Interrupt::is_enabled());
            self.task_queue.lock().push_back(task.id());
        }}
    }

    pub fn unblock_receiving_task(&self, id: TaskId) {
        uninterruptable! {{
            let task = self.get_task_by_id(id).unwrap();
            assert!(task.scheduler_state().borrow().run_state == RunState::Receiving);
            // Add this task to ready queue
            task.scheduler_state().borrow_mut().run_state = RunState::Ready;
            debug_assert!(!Target::Interrupt::is_enabled());
            self.task_queue.lock().push_back(task.id());
        }}
    }

    pub fn block_current_task_as_sending(&self) {
        uninterruptable! {{
            let task = self.get_current_task().unwrap();
            assert!(task.scheduler_state().borrow().run_state == RunState::Running);
            task.scheduler_state().borrow_mut().run_state = RunState::Sending;
        }}
        // self.schedule();
        unimplemented!()
    }

    pub fn block_current_task_as_receiving(&self) -> Message {
        uninterruptable! {{
            let task = self.get_current_task().unwrap();
            assert!(task.scheduler_state().borrow().run_state == RunState::Running, "{:?} {:?}", task.id(), task.scheduler_state().borrow().run_state);
            task.scheduler_state().borrow_mut().run_state = RunState::Receiving;
            self.try_schedule();
        }}
        // self.schedule();
        // unimplemented!()
    }

    pub fn enqueue_current_task_as_ready(&self) {
        debug_assert!(!Target::Interrupt::is_enabled());
        let task = self.get_current_task().unwrap();
        // println!("Enqueue {:?} as ready", task.id());
        assert!(task.scheduler_state().borrow().run_state != RunState::Ready);
        task.scheduler_state().borrow_mut().run_state = RunState::Ready;
        self.task_queue.lock().push_back(task.id());
    }

    fn get_next_schedulable_task(&self) -> &'static mut Task {
        debug_assert!(!Target::Interrupt::is_enabled());
        if let Some(next_runnable_task) = self.task_queue.lock().pop_front() {
            Task::by_id(next_runnable_task).expect("task not found")
        } else {
            // println!("No task to schedule");
            if let Some(current_task) = self.get_current_task() {
                // println!("No task to schedule");
                uninterruptable! {{
                    let mut state = current_task.scheduler_state().borrow_mut();
                    state.time_slice_units = 100;
                }};
                current_task
            } else {
                // println!("Nothing to schedule 1!");
                panic!()
            }
        }
    }

    pub fn try_schedule(&self) -> ! {
        // println!("s");
        Target::Interrupt::disable();

        let current_task = self.get_current_task();

        if current_task.is_some() && current_task.as_ref().unwrap().scheduler_state().borrow().run_state == RunState::Running {
            // Continue with this task
            unsafe { current_task.unwrap().context.return_to_user(); }
        } else {
            // Current Task is blocked, switch to a new task

            // Find a scheduleable task
            let next_task = self.get_next_schedulable_task();
            // if Some(next_task.id()) == current_task.as_ref().map(|t| t.id()) {
            //     println!("!!! Nothing to schedule");
            //     {
            //         let mut state = next_task.scheduler_state().borrow_mut();
            //         state.run_state = RunState::Running;
            //         state.time_slice_units = 100;
            //     }
            //     unsafe { current_task.unwrap().context.return_to_user(); }
            // }
            // debug_assert!(Some(next_task.id()) != current_task.as_ref().map(|t| t.id()));
            debug_assert!({
                let mut state = next_task.scheduler_state().borrow_mut();
                state.run_state == RunState::Ready
            });
            println!("Switch: {:?} -> {:?}", current_task.as_ref().map(|t| t.id()), next_task.id());
            // Run next task
            {
                let mut state = next_task.scheduler_state().borrow_mut();
                state.run_state = RunState::Running;
                state.time_slice_units = 100;
            }
            self.set_current_task_id(next_task.id());
            unsafe { next_task.context.return_to_user(); }
        }
    }

    pub fn timer_tick(&self) {
        // unsafe { self.get_current_task().unwrap().context.return_to_user(); }
        // print!(".");
        
        debug_assert!(!Target::Interrupt::is_enabled());
        let current_task = match self.get_current_task() {
            Some(t) => t,
            None => {
                println!("tsu is 0");
                panic!("No current task!");
                return
            }
        };
        if current_task.scheduler_state().borrow().time_slice_units == 0 {
            println!("tsu is 0");
            panic!("time_slice_units is zero");
            // return;
        }
        {
            let mut scheduler_state = current_task.scheduler_state().borrow_mut();
            debug_assert!(scheduler_state.run_state == RunState::Running, "Invalid state {:?} for {:?}", scheduler_state.run_state, current_task.id());
            scheduler_state.time_slice_units -= 1;
            if scheduler_state.time_slice_units == 0 {
                scheduler_state.time_slice_units = 100;
                ::core::mem::drop(scheduler_state);
                // print!(",");
                self.enqueue_current_task_as_ready();
                
                // print!("!");
                // println!("t");
                self.try_schedule();
            } else {
                ::core::mem::drop(scheduler_state);
                unsafe { self.get_current_task().unwrap().context.return_to_user() }
            }
        }
        
        debug_assert!(!Target::Interrupt::is_enabled());
    }
}

unsafe impl Send for Scheduler {}
unsafe impl Sync for Scheduler {}
