use alloc::collections::BTreeSet;
use spin::Mutex;
use core::sync::atomic::{AtomicUsize, Ordering};
use super::scheduler::*;
use core::cell::RefCell;
use crate::*;
pub use proton::{IPC, TaskId, Message};
use alloc::boxed::Box;
use crate::kernel_process::KernelTask;

static TASK_ID_COUNT: AtomicUsize = AtomicUsize::new(0);


#[derive(Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Clone, Copy)]
pub enum TaskState {
    Ready,
    Running,
    Blocked,
}

pub struct Task<K: AbstractKernel> {
    id: TaskId,
    scheduler_state: RefCell<<K::Scheduler as AbstractScheduler>::State>,
    pub context: <K::Arch as AbstractArch>::Context,
    pub block_to_receive_from: Mutex<Option<Option<TaskId>>>,
    block_to_send: Option<Message>,
    blocked_senders: Mutex<BTreeSet<TaskId>>,
}

impl <K: AbstractKernel> Task<K> {
    #[inline]
    pub fn id(&self) -> TaskId {
        self.id
    }

    #[inline]
    pub fn scheduler_state(&self) -> &RefCell<<K::Scheduler as AbstractScheduler>::State> {
        &self.scheduler_state
    }

    #[inline]
    pub fn receive_message(from: Option<TaskId>) -> ! {
        let receiver = Task::<K>::current().unwrap();
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
                let sender = Task::<K>::by_id(sender_id).unwrap();
                let m = sender.block_to_send.take().unwrap();
                K::global().scheduler.unblock_sending_task(sender_id, 0);
                // We've received a message, return to user program
                receiver.context.set_response_message(m);
                receiver.context.set_response_status(0);
                K::global().scheduler.schedule();
            }
        }
        // Block receiver
        *receiver.block_to_receive_from.lock() = Some(from);
        K::global().scheduler.block_current_task_as_receiving();
    }

    #[inline]
    pub fn send_message(m: Message) -> ! {
        let sender = Task::<K>::by_id(m.sender).unwrap();
        debug_assert!(sender.id() == Task::<K>::current().unwrap().id());
        let receiver = Task::<K>::by_id(m.receiver).unwrap();
        // If the receiver is blocked for this sender, copy message & unblock the receiver
        {
            let mut block_to_receive_from_guard = receiver.block_to_receive_from.lock();
            if let Some(block_to_receive_from) = *block_to_receive_from_guard {
                if block_to_receive_from.is_none() || block_to_receive_from == Some(sender.id) {
                    debug!(K: "Unblock {:?} for message {:?}", receiver.id, m);
                    *block_to_receive_from_guard = None;
                    K::global().scheduler.unblock_receiving_task(receiver.id, 0, m);
                    // Succesfully send the message, return to user
                    sender.context.set_response_status(0);
                    debug!(K: "Sender: {:?}", sender.scheduler_state.borrow());
                    ::core::mem::drop(block_to_receive_from_guard);
                    K::global().scheduler.schedule()
                }
            }
        }
        // Else, block the sender until message is delivered
        {
            sender.block_to_send = Some(m);
            let mut blocked_senders = receiver.blocked_senders.lock();
            blocked_senders.insert(sender.id);
        }
        K::global().scheduler.block_current_task_as_sending();
    }

    /// Fork a new task.
    /// This will duplicate the virtual memory
    // pub fn fork(&self) -> &'static mut Task {
    //     let id = TaskId(TASK_ID_COUNT.fetch_add(1, Ordering::SeqCst));
    //     // Allocate task struct
    //     let task = box Task {
    //         id,
    //         context: self.context.fork(),
    //         scheduler_state: self.scheduler_state.clone(),
    //         block_to_receive_from: Mutex::new(*self.block_to_receive_from.lock()),
    //         block_to_send: None,
    //         blocked_senders: Mutex::new(BTreeSet::new()),
    //     };
    //     GLOBAL_TASK_SCHEDULER.register_new_task(task)
    // }
    /// Create a init task with empty p4 table
    pub fn create_kernel_task(t: Box<dyn KernelTask>) -> &'static mut Self {
        debug!(K: "create_kernel_task 1");
        let t = box t;
        debug!(K: "create_kernel_task 2");
        // Assign an id
        let id = TaskId(TASK_ID_COUNT.fetch_add(1, Ordering::SeqCst));
        debug!(K: "create_kernel_task 3");
        // Alloc task struct
        let task = box Task {
            id,
            context: <K::Arch as AbstractArch>::Context::new(entry as _, Box::into_raw(t) as usize as *mut ()),
            scheduler_state: RefCell::new(Default::default()),
            block_to_receive_from: Mutex::new(None),
            block_to_send: None,
            blocked_senders: Mutex::new(BTreeSet::new()),
        };
        debug!(K: "create_kernel_task 4");
        // Add this task to the scheduler
        K::global().scheduler.register_new_task(task)
    }

    pub fn create_kernel_task2(_t: Box<dyn KernelTask>) {
        // let t = Box::leak(box t);
        // Assign an id
        // let id = TaskId(TASK_ID_COUNT.fetch_add(1, Ordering::SeqCst));
        // Alloc task struct
        // let task = box Task::<K> {
        //     id,
        //     context: <K::Arch as AbstractArch>::Context::new(entry as _, Box::into_raw(t) as usize as *mut ()),
        //     scheduler_state: RefCell::new(Default::default()),
        //     block_to_receive_from: Mutex::new(None),
        //     block_to_send: None,
        //     blocked_senders: Mutex::new(BTreeSet::new()),
        // };
         <K::Arch as AbstractArch>::Context::new2();
        // Add this task to the scheduler
        // K::global().scheduler.register_new_task(task)
    }

    pub fn by_id(id: TaskId) -> Option<&'static mut Self> {
        K::global().scheduler.get_task_by_id(id)
    }

    pub fn current() -> Option<&'static mut Self> {
        K::global().scheduler.get_current_task()
    }
}

unsafe impl <K: AbstractKernel> Send for Task<K> {}
unsafe impl <K: AbstractKernel> Sync for Task<K> {}

impl <K: AbstractKernel> PartialEq for Task<K> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl <K: AbstractKernel> Eq for Task<K> {}

extern fn entry(t: *mut Box<dyn KernelTask>) -> ! {
    let mut t: Box<Box<dyn KernelTask>> = unsafe { Box::from_raw(t) };
    t.run()
}