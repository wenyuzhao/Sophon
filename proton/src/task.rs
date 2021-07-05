use super::scheduler::*;
use crate::arch::Arch;
use crate::arch::ArchContext;
use crate::arch::TargetArch;
use crate::scheduler::ipc::IPC;
use crate::*;
use alloc::boxed::Box;
use alloc::collections::BTreeSet;
use core::cell::RefCell;
use core::sync::atomic::{AtomicUsize, Ordering};
use kernel_tasks::KernelTask;
use spin::Mutex;
// use crate::kernel_process::KernelTask;

static TASK_ID_COUNT: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Clone, Copy)]
pub enum TaskState {
    Ready,
    Running,
    Blocked,
}

#[derive(Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Clone, Copy)]
pub struct TaskId(pub usize);

impl TaskId {
    pub const NULL: Self = Self(0);
    pub const KERNEL: Self = Self(0);
}

#[repr(C, align(64))]
#[derive(Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Clone)]
pub struct Message {
    pub sender: TaskId,
    pub receiver: TaskId, // None for all tasks
    pub kind: usize,
    data: [u64; 5],
}

impl Message {
    #[inline]
    pub fn new(sender: TaskId, receiver: TaskId, kind: usize) -> Self {
        Self {
            sender,
            receiver,
            kind,
            data: [0; 5],
        }
    }

    #[inline]
    pub fn with_data<T>(mut self, data: T) -> Self {
        self.set_data(data);
        self
    }

    #[inline]
    pub fn set_data<T>(&mut self, data: T) {
        debug_assert!(::core::mem::size_of::<T>() <= ::core::mem::size_of::<[u64; 5]>());
        unsafe {
            let data_ptr: *mut T = &mut self.data as *mut [u64; 5] as usize as *mut T;
            data_ptr.write(data);
        }
    }

    #[inline]
    pub fn get_data<T>(&self) -> &T {
        debug_assert!(::core::mem::size_of::<T>() <= ::core::mem::size_of::<[u64; 5]>());
        unsafe { ::core::mem::transmute(&self.data) }
    }

    #[inline]
    pub fn send(self) {
        // IPC::send(self);
        unimplemented!()
    }

    #[inline]
    pub fn receive(src: Option<TaskId>) -> Message {
        IPC::receive(src)
    }

    #[inline]
    pub fn reply<T>(&self, data: T) {
        let m = Message::new(self.receiver, self.sender, self.kind).with_data(data);
        // IPC::send(n);
        unimplemented!("{:?}", m)
    }
}

pub struct Task {
    id: TaskId,
    scheduler_state: RefCell<<Scheduler as AbstractScheduler>::State>,
    pub context: <TargetArch as Arch>::Context,
    pub block_to_receive_from: Mutex<Option<Option<TaskId>>>,
    block_to_send: Option<Message>,
    blocked_senders: Mutex<BTreeSet<TaskId>>,
}

impl Task {
    #[inline]
    pub fn id(&self) -> TaskId {
        self.id
    }

    #[inline]
    pub fn scheduler_state<S: AbstractScheduler>(&self) -> &RefCell<S::State> {
        unsafe {
            &*(&self.scheduler_state as *const RefCell<<Scheduler as AbstractScheduler>::State>
                as *const RefCell<S::State>)
        }
    }

    #[inline]
    pub fn receive_message(from: Option<TaskId>) -> ! {
        let receiver = Task::current().unwrap();
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
                SCHEDULER.unblock_sending_task(sender_id, 0);
                // We've received a message, return to user program
                receiver.context.set_response_message(m);
                receiver.context.set_response_status(0);
                SCHEDULER.schedule();
            }
        }
        // Block receiver
        *receiver.block_to_receive_from.lock() = Some(from);
        SCHEDULER.block_current_task_as_receiving();
    }

    #[inline]
    pub fn send_message(m: Message) -> ! {
        let sender = Task::by_id(m.sender).unwrap();
        debug_assert!(sender.id() == Task::current().unwrap().id());
        let receiver = Task::by_id(m.receiver).unwrap();
        // If the receiver is blocked for this sender, copy message & unblock the receiver
        {
            let mut block_to_receive_from_guard = receiver.block_to_receive_from.lock();
            if let Some(block_to_receive_from) = *block_to_receive_from_guard {
                if block_to_receive_from.is_none() || block_to_receive_from == Some(sender.id) {
                    log!("Unblock {:?} for message {:?}", receiver.id, m);
                    *block_to_receive_from_guard = None;
                    SCHEDULER.unblock_receiving_task(receiver.id, 0, m);
                    // Succesfully send the message, return to user
                    sender.context.set_response_status(0);
                    log!("Sender: {:?}", sender.scheduler_state.borrow());
                    ::core::mem::drop(block_to_receive_from_guard);
                    SCHEDULER.schedule()
                }
            }
        }
        // Else, block the sender until message is delivered
        {
            sender.block_to_send = Some(m);
            let mut blocked_senders = receiver.blocked_senders.lock();
            blocked_senders.insert(sender.id);
        }
        SCHEDULER.block_current_task_as_sending();
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
        log!("create_kernel_task 1");
        let t = box t;
        log!("create_kernel_task 2");
        // Assign an id
        let id = TaskId(TASK_ID_COUNT.fetch_add(1, Ordering::SeqCst));
        log!("create_kernel_task 3");
        // Alloc task struct
        let task = box Task {
            id,
            context: <TargetArch as Arch>::Context::new(
                entry as _,
                Box::into_raw(t) as usize as *mut (),
            ),
            scheduler_state: RefCell::new(Default::default()),
            block_to_receive_from: Mutex::new(None),
            block_to_send: None,
            blocked_senders: Mutex::new(BTreeSet::new()),
        };
        log!("create_kernel_task 4");
        // Add this task to the scheduler
        SCHEDULER.register_new_task(task)
    }

    pub fn by_id(id: TaskId) -> Option<&'static mut Self> {
        SCHEDULER.get_task_by_id(id)
    }

    pub fn current() -> Option<&'static mut Self> {
        SCHEDULER.get_current_task()
    }

    pub fn get_context<C: ArchContext>(&mut self) -> &mut C {
        let ptr = &mut self.context as *mut _;
        unsafe { &mut *(ptr as *mut C) }
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

extern "C" fn entry(t: *mut Box<dyn KernelTask>) -> ! {
    let mut t: Box<Box<dyn KernelTask>> = unsafe { Box::from_raw(t) };
    t.run()
}
