use alloc::boxed::Box;
use alloc::collections::{BTreeMap, BTreeSet, LinkedList};
use spin::Mutex;
use core::sync::atomic::{AtomicUsize, Ordering};
use alloc::vec::Vec;
use super::scheduler::*;
use core::cell::{RefMut, RefCell};
use crate::mm::*;
use crate::mm::heap_constants::*;
use crate::utils::atomic_queue::AtomicQueue;
use crate::arch::*;
use Target::Context;

use core::iter::Step;



static TASK_ID_COUNT: AtomicUsize = AtomicUsize::new(0);


#[derive(Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Clone, Copy)]
pub enum TaskState {
    Ready,
    Running,
    Blocked,
}

#[derive(Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Clone, Copy)]
pub struct TaskId(usize);


#[derive(Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Clone, Copy)]
pub struct Message {
    pub sender: TaskId,
    pub receiver: TaskId, // None for all tasks
    pub kind: usize,
    pub data: [u64; 16],
}

pub struct Task {
    id: TaskId,
    scheduler_state: RefCell<SchedulerState>,
    pub context: Context,
    pub block_to_receive_from: Mutex<Option<Option<TaskId>>>,
    pub incoming_message: Option<Message>,
    block_to_send: Option<Message>,
    blocked_senders: Mutex<BTreeSet<TaskId>>,
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

    #[inline]
    pub fn receive_message(from: Option<TaskId>, slot: &mut Message) {
        let receiver = Task::current().unwrap();
        // println!("{:?} waiting for {:?}", receiver.id, from);
        // Search from blocked_senders
        {
            let mut blocked_senders = receiver.blocked_senders.lock();
            let mut target_sender = None;
            for tid in blocked_senders.iter() {
                if from.is_none() || Some(*tid) == from {
                    target_sender = Some(*tid);
                }
            }
            if let Some(sender_id) = target_sender {
                // Unblock this sender
                blocked_senders.remove(&sender_id);
                let sender = Task::by_id(sender_id).unwrap();
                let m = sender.block_to_send.take().unwrap();
                GLOBAL_TASK_SCHEDULER.unblock_sending_task(sender_id);
                *slot = m;
                return;
                // return m;
            }
        }
        // Block receiver
        *receiver.block_to_receive_from.lock() = Some(from);
        GLOBAL_TASK_SCHEDULER.block_current_task_as_receiving();
        let t = Task::current().unwrap();
        *slot = t.incoming_message.unwrap();
        return;//t.incoming_message.take().unwrap();
    }
    
    #[inline]
    pub fn send_message(m: Message) {
        let sender = Task::by_id(m.sender).unwrap();
        let receiver = Task::by_id(m.receiver).unwrap();
        // println!("{:?} -> {:?}", sender.id, receiver.id);
        // If the receiver is blocked for this sender, copy message & unblock the receiver
        {
            let mut block_to_receive_from_guard = receiver.block_to_receive_from.lock();
            if let Some(block_to_receive_from) = *block_to_receive_from_guard {
                // println!("Receiver is blocked");
                if block_to_receive_from.is_none() || block_to_receive_from == Some(sender.id) {
                    receiver.incoming_message = Some(m);
                    *block_to_receive_from_guard = None;
                    println!("Unblock {:?} for message {:?}", receiver.id, m);
                    GLOBAL_TASK_SCHEDULER.unblock_receiving_task(receiver.id);
                    return
                }
            }
        }
        // Else, block the sender until message is delivered
        {
            sender.block_to_send = Some(m);
            let mut blocked_senders = receiver.blocked_senders.lock();
            blocked_senders.insert(sender.id);
        }
        GLOBAL_TASK_SCHEDULER.block_current_task_as_sending();
    }

    /// Fork a new task.
    /// This will duplicate the virtual memory
    pub fn fork(&self) -> &'static mut Task {
        let id = TaskId(TASK_ID_COUNT.fetch_add(1, Ordering::SeqCst));
        // Allocate task struct
        let task = box Task {
            id,
            context: self.context.fork(),
            scheduler_state: self.scheduler_state.clone(),
            block_to_receive_from: Mutex::new(*self.block_to_receive_from.lock()),
            incoming_message: None,
            block_to_send: None,
            blocked_senders: Mutex::new(BTreeSet::new()),
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
            block_to_receive_from: Mutex::new(None),
            incoming_message: None,
            block_to_send: None,
            blocked_senders: Mutex::new(BTreeSet::new()),
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
        Target::Interrupt::enable();
        unsafe {
            if let Some(from_task) = from_task {
                from_task.context.switch_to(&to_task.context);
            } else {
                let mut temp_ctx = Context::empty();
                temp_ctx.switch_to(&to_task.context);
            }
        }
        // crate::interrupt::disable();
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
