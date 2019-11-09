use alloc::boxed::Box;
use alloc::collections::{BTreeMap, LinkedList};
use super::context::*;
use spin::Mutex;
use core::sync::atomic::{AtomicUsize, Ordering};
use alloc::vec::Vec;
use super::scheduler::*;
use core::cell::{RefMut, RefCell};
use crate::mm::*;
use crate::exception::ExceptionFrame;
use crate::mm::heap_constants::*;

use core::iter::Step;



static TASK_ID_COUNT: AtomicUsize = AtomicUsize::new(1);


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
    scheduler_state: RefCell<SchedulerState>,
    context: Context,
    kernel_stack: [Frame<Size4K>; KERNEL_STACK_PAGES],
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

    #[inline(never)]
    pub fn fork(&self, parent_sp: usize) -> &'static mut Task {
        let id = TaskId(TASK_ID_COUNT.fetch_add(1, Ordering::SeqCst));
        // debug!("parent sp = {:x}", parent_sp);
        // debug!("parent sp bottom = {:?}", self.kernel_stack);
        // let parent_sp_offset = parent_sp - self.kernel_stack.start().as_usize();
        // // let kernel_stack = self.kernel_stack.clone();
        // let stack_pointer = kernel_stack.start().as_usize() + parent_sp_offset;
        // debug!("child sp = {:x}", stack_pointer);
        // debug!("child sp bottom = {:?}", kernel_stack);
        let mut task = box Task {
            id,
            context: self.context.clone(),
            kernel_stack: {
                let mut frames = [Frame::ZERO; KERNEL_STACK_PAGES];
                for i in 0..KERNEL_STACK_PAGES {
                    frames[i] = frame_allocator::alloc().unwrap();
                }
                frames
            },
            scheduler_state: RefCell::new(SchedulerState::new()),
        };
        debug!("A");
        // let stack_frame = frame_allocator::alloc::<Size2M>().unwrap();
        // task.kernal_stack_frame
        task.context.sp = parent_sp as _;
        task.context.pc = crate::exception::exit_from_exception as _;
        task.context.p4 = {
            let mut stacks = [(Frame::ZERO, Frame::ZERO); KERNEL_STACK_PAGES];
            for i in 0..KERNEL_STACK_PAGES {
                stacks[i] = (self.kernel_stack[i], task.kernel_stack[i]);
            }
            debug!("B {:?}", stacks);
            paging::fork_page_table(self.context.p4, &stacks)
        };
        debug!("C");
        // {unsafe {
        //     let page = crate::mm::map_kernel_temporarily(task.context.p4, PageFlags::OUTER_SHARE | PageFlags::ACCESSED | PageFlags::SMALL_PAGE);
        //     let new_table = unsafe { page.start().as_ref_mut::<PageTable<L4>>() };
        //     debug!("p4({:?})[0] = {:?} {:?}", task.context.p4, new_table.entries[0].address(), new_table.entries[0].flags());
        //     debug!("p4({:?})[511] = {:?} {:?}", task.context.p4, new_table.entries[511].address(), new_table.entries[511].flags());
        // }}
        // unsafe {
        //     let f = Frame::new(0x120c000usize.into());
        //     let page = crate::mm::map_kernel_temporarily::<Size4K>(f, PageFlags::OUTER_SHARE | PageFlags::ACCESSED | PageFlags::SMALL_PAGE);
        //     let new_table = unsafe { page.start().as_ref_mut::<PageTable<L2>>() };
        //     debug!("p3({:?})[0] = {:?} {:?}", f, new_table.entries[0].address(), new_table.entries[0].flags());
        //     debug!("p3({:?})[511] = {:?} {:?}", f, new_table.entries[511].address(), new_table.entries[511].flags());
        // }
        // unsafe {
        //     let f = Frame::new(0x120d000usize.into());
        //     let page = crate::mm::map_kernel_temporarily::<Size4K>(f, PageFlags::OUTER_SHARE | PageFlags::ACCESSED | PageFlags::SMALL_PAGE);
        //     let new_table = unsafe { page.start().as_ref_mut::<PageTable<L2>>() };
        //     debug!("p2({:?})[0] = {:?} {:?}", f, new_table.entries[0].address(), new_table.entries[0].flags());
        //     debug!("p2({:?})[1] = {:?} {:?}", f, new_table.entries[1].address(), new_table.entries[1].flags());
        //     debug!("p2({:?})[511] = {:?} {:?}", f, new_table.entries[511].address(), new_table.entries[511].flags());
        // }

        // Copy stack
        for i in 0..KERNEL_STACK_PAGES {
            let parent_stack_page = crate::mm::map_kernel_temporarily2(self.kernel_stack[i], PageFlags::OUTER_SHARE | PageFlags::ACCESSED | PageFlags::SMALL_PAGE, Some(0xffff_1111_2222_2000));
            let child_stack_page = crate::mm::map_kernel_temporarily2(task.kernel_stack[i], PageFlags::OUTER_SHARE | PageFlags::ACCESSED | PageFlags::SMALL_PAGE, Some(0xffff_1111_2222_3000));
            debug!("{:?} {:?}", *parent_stack_page, *child_stack_page);
            let mut cursor = 0;
            while cursor < (1usize << Size4K::LOG_SIZE) {
                unsafe {
                    (child_stack_page.start() + cursor).store::<usize>((parent_stack_page.start() + cursor).load());
                }
                cursor += 8;
            }
        }
        debug!("D");
        // Set child process return value (x0)
        {
            let sp_offset = parent_sp - KERNEL_STACK_START.as_usize();
            let page_index = sp_offset >> Size4K::LOG_SIZE;
            let page_offset = sp_offset & ((1 << Size4K::LOG_SIZE) - 1);
            let stack_page = crate::mm::map_kernel_temporarily(task.kernel_stack[page_index], PageFlags::OUTER_SHARE | PageFlags::ACCESSED | PageFlags::SMALL_PAGE);
            let child_exception_frame_ptr = stack_page.start() + page_offset;
            let child_exception_frame = unsafe { child_exception_frame_ptr.as_ref_mut::<ExceptionFrame>() };
            child_exception_frame.x0 = 0;
        }
        // unsafe {
        //     let stack_page = crate::mm::map_kernel_temporarily(stack_frame, PageFlags::OUTER_SHARE | PageFlags::ACCESSED);
        //     let mut cursor = 0;
        //     while cursor < (1usize << Size2M::LOG_SIZE) {
        //         (stack_page.start() + cursor).store::<usize>((self.kernel_stack.0.start() + cursor).load());
        //         cursor += 8;
        //     }
        //     // Set child process return value (x0)
        //     let sp_offset = parent_sp - self.kernel_stack.0.start().as_usize();
        //     let child_exception_frame_ptr = stack_page.start() + sp_offset;
        //     let child_exception_frame = child_exception_frame_ptr.as_ref_mut::<ExceptionFrame>();
        //     child_exception_frame.x0 = 233;
        // }
        // PageTable::<L4>::with_temporary_low_table(self.context.p4, |p4| {
        //     p4.update_flags(self.kernel_stack, PageFlags::ACCESSED | PageFlags::OUTER_SHARE | PageFlags::PRESENT);
        // });
        // PageTable::<L4>::with_temporary_low_table(task.context.p4, |p4| {
        //     loop {}
        //     // p4.remap(task.kernel_stack, stack_frame, PageFlags::ACCESSED | PageFlags::OUTER_SHARE | PageFlags::PRESENT);
        // });
        // loop {}
        // Give it a new kernel stack
        GLOBAL_TASK_SCHEDULER.register_new_task(task)
    }
}

impl Task {
    /// Create a init task with empty p4 table
    pub fn create_init_task(entry: extern fn() -> !) -> &'static mut Task {
        // Alloc task struct
        let id = TaskId(TASK_ID_COUNT.fetch_add(1, Ordering::SeqCst));
        // Alloc page table
        let p4_frame = frame_allocator::alloc::<Size4K>().unwrap();
        unsafe {
            let p4_page = crate::mm::map_kernel_temporarily(p4_frame, PageFlags::OUTER_SHARE | PageFlags::ACCESSED);
            let p4 = p4_page.start().as_ref_mut::<PageTable<L4>>();
            p4.entries[511].set(p4_frame, PageFlags::SMALL_PAGE | PageFlags::OUTER_SHARE | PageFlags::ACCESSED | PageFlags::PRESENT);
        }
        // Alloc & map stack
        let mut kernal_stack_frames = [Frame::ZERO; KERNEL_STACK_PAGES];
        for i in 0..KERNEL_STACK_PAGES {
            let stack_frame = frame_allocator::alloc::<Size4K>().unwrap();
            let stack_page = Page::of(KERNEL_STACK_START).add_usize(i).unwrap();
            debug!("stack_page = {:?}", stack_page);
            kernal_stack_frames[i] = stack_frame;
            PageTable::<L4>::with_temporary_low_table(p4_frame, |p4| {
                p4.map(stack_page, stack_frame, PageFlags::ACCESSED | PageFlags::OUTER_SHARE | PageFlags::PRESENT | PageFlags::SMALL_PAGE);
            });
        }
        debug!("kernal_stack_frames {:?}", kernal_stack_frames);
        
        let mut task = box Task {
            id,
            context: Context::new(entry as _, KERNEL_STACK_END.as_ptr_mut()),
            kernel_stack: kernal_stack_frames,
            scheduler_state: RefCell::new(SchedulerState::new()),
        };
        task.context.p4 = p4_frame;

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
        crate::interrupt::enable();
        unsafe {
            if let Some(from_task) = from_task {
                from_task.context.switch_to(&to_task.context);
            } else {
                let mut temp_ctx = Context::empty();
                // temp_ctx.p4 = unsafe {
                //     use cortex_a::regs::*;
                //     Frame::new((TTBR0_EL1.get() as usize).into())
                // };
                temp_ctx.switch_to(&to_task.context);
            }
        }
        crate::interrupt::disable();
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
