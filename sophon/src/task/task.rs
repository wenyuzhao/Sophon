use super::scheduler::AbstractScheduler;
use super::scheduler::Scheduler;
use super::scheduler::SCHEDULER;
use super::Message;
use super::TaskId;
use crate::arch::Arch;
use crate::arch::ArchContext;
use crate::arch::TargetArch;
use crate::memory::kernel::KERNEL_MEMORY_MAPPER;
use crate::utils::unint_lock::UnintMutex;
use crate::*;
use ::memory::address::Address;
use ::memory::address::V;
use ::memory::page::Page;
use ::memory::page::PageSize;
use ::memory::page::Size4K;
use ::memory::page_table::{PageFlags, PageFlagsExt};
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use atomic::Atomic;
use core::cell::RefCell;
use core::iter::Step;
use core::ops::Range;
use core::sync::atomic::{AtomicUsize, Ordering};
use ipc::scheme::Resource;
use ipc::scheme::SchemeId;
use kernel_tasks::KernelTask;
use spin::Mutex;

static TASK_ID_COUNT: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Clone, Copy)]
pub enum TaskState {
    Ready,
    Running,
    Blocked,
}

pub struct Task {
    pub id: TaskId,
    scheduler_state: RefCell<<Scheduler as AbstractScheduler>::State>,
    pub context: <TargetArch as Arch>::Context,
    pub block_to_receive_from: Mutex<Option<Option<TaskId>>>,
    block_to_send: UnintMutex<Option<Message>>,
    blocked_senders: Mutex<BTreeSet<TaskId>>,
    pub resources: Mutex<BTreeMap<Resource, SchemeId>>,
    virtual_memory_highwater: Atomic<Address<V>>,
    pub proc: &'static Proc,
}

impl Task {
    #[inline]
    pub fn scheduler_state<S: AbstractScheduler>(&self) -> &RefCell<S::State> {
        unsafe {
            &*(&self.scheduler_state as *const RefCell<<Scheduler as AbstractScheduler>::State>
                as *const RefCell<S::State>)
        }
    }

    #[inline]
    pub fn receive_message(from: Option<TaskId>) -> ! {
        let receiver = Task::current();
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
                let m = sender.block_to_send.lock().take().unwrap();
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
        debug_assert!(sender.id == Task::current().id);
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
            *sender.block_to_send.lock() = Some(m);
            let mut blocked_senders = receiver.blocked_senders.lock();
            blocked_senders.insert(sender.id);
        }
        SCHEDULER.block_current_task_as_sending();
    }

    pub(super) fn create(proc: &'static mut Proc, t: Box<dyn KernelTask>) -> Box<Self> {
        let t = Box::into_raw(box t);
        let id = TaskId(TASK_ID_COUNT.fetch_add(1, Ordering::SeqCst));
        box Task {
            id,
            context: <TargetArch as Arch>::Context::new(entry as _, t as *mut ()),
            scheduler_state: RefCell::new(Default::default()),
            block_to_receive_from: Mutex::new(None),
            block_to_send: UnintMutex::new(None),
            blocked_senders: Mutex::new(BTreeSet::new()),
            resources: Mutex::new(BTreeMap::new()),
            virtual_memory_highwater: Atomic::new(crate::memory::USER_SPACE_MEMORY_RANGE.start),
            proc,
        }
    }

    pub fn by_id(id: TaskId) -> Option<&'static Self> {
        SCHEDULER.get_task_by_id(id)
    }

    pub fn current() -> &'static Self {
        SCHEDULER.get_current_task().unwrap()
    }

    pub fn get_context<C: ArchContext>(&self) -> &C {
        let ptr = &self.context as *const _;
        unsafe { &mut *(ptr as *mut C) }
    }

    pub fn sbrk(&self, num_pages: usize) -> Option<Range<Page<Size4K>>> {
        let result =
            self.virtual_memory_highwater
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |old| {
                    let old_aligned = old.align_up(Size4K::BYTES);
                    Some(old_aligned + (num_pages << Size4K::LOG_BYTES))
                });
        log!("sbrk: {:?} {:?}", self.id, result);
        match result {
            Ok(a) => {
                let old_top = a;
                let start = Page::new(a.align_up(Size4K::BYTES));
                let end = Page::forward(start, num_pages);
                debug_assert_eq!(old_top, start.start());
                // Map old_top .. end
                {
                    let page_table = self.proc.get_page_table();
                    let _guard = KERNEL_MEMORY_MAPPER.with_kernel_page_table();
                    for page in start..end {
                        let frame = PHYSICAL_MEMORY.acquire().unwrap();
                        page_table.map(
                            page,
                            frame,
                            PageFlags::user_data_flags_4k(),
                            &PHYSICAL_MEMORY,
                        );
                    }
                }
                Some(start..end)
            }
            Err(_e) => return None,
        }
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
