use super::raw_module_call;
use super::MODULES;
use crate::arch::ArchContext;
use crate::arch::{Arch, TargetArch};
use crate::memory::kernel::KERNEL_HEAP;
use crate::memory::kernel::KERNEL_MEMORY_MAPPER;
use crate::modules::SCHEDULER;
use crate::task::Proc;
use crate::task::Task;
use crate::utils::testing::Tests;
use alloc::boxed::Box;
use core::alloc::GlobalAlloc;
use core::any::Any;
use core::iter::Step;
use core::ops::Range;
use device_tree::DeviceTree;
use kernel_module::ModuleCallHandler;
use log::Logger;
use memory::page::Frame;
use memory::page_table::PageFlags;
use memory::{
    address::Address,
    page::{Page, Size4K},
};
use proc::{ProcId, TaskId};

pub struct KernelService(pub usize);

impl kernel_module::KernelService for KernelService {
    fn log(&self, s: &str) {
        print!("{}", s);
    }

    fn set_sys_logger(&self, logger: &'static dyn Logger) {
        log::init(logger)
    }

    fn register_tests(&self, tests: Tests) {
        crate::utils::testing::register_kernel_tests(tests);
    }

    fn alloc(&self, layout: core::alloc::Layout) -> Option<Address> {
        let ptr = unsafe { crate::ALLOCATOR.alloc(layout) };
        if ptr.is_null() {
            None
        } else {
            Some(ptr.into())
        }
    }

    fn dealloc(&self, ptr: Address, layout: core::alloc::Layout) {
        unsafe { crate::ALLOCATOR.dealloc(ptr.as_mut_ptr(), layout) }
    }

    fn register_module_call_handler(&self, handler: &'static dyn ModuleCallHandler) {
        // log!("register module call");
        MODULES.write()[self.0].as_mut().map(|m| {
            m.call = Some(handler);
        });
    }

    fn module_call<'a>(&self, module: &str, request: syscall::RawModuleRequest<'a>) -> isize {
        raw_module_call(module, true, request.as_buf())
    }

    fn current_process(&self) -> Option<ProcId> {
        Proc::current_opt().map(|p| p.id)
    }

    fn current_task(&self) -> Option<proc::TaskId> {
        Task::current_opt().map(|p| p.id)
    }

    fn handle_panic(&self) -> ! {
        if cfg!(sophon_test) {
            TargetArch::halt(-1)
        }
        syscall::exit();
    }
    fn vfs(&self) -> &'static dyn vfs::VFSManager {
        &*crate::modules::VFS
    }

    fn get_vfs_state(&self, proc: ProcId) -> &dyn Any {
        let proc = Proc::by_id(proc).unwrap();
        unsafe { &*(proc.fs.as_ref() as *const dyn Any) }
    }

    fn set_vfs_manager(&self, vfs_manager: &'static dyn vfs::VFSManager) {
        crate::modules::VFS.set_vfs_manager(vfs_manager);
        vfs_manager.init(unsafe { &mut *crate::INIT_FS.unwrap() });
    }

    fn get_device_tree(&self) -> Option<&'static DeviceTree<'static, 'static>> {
        unsafe { crate::DEV_TREE.as_ref() }
    }

    fn map_device_page(&self, frame: Frame) -> Page {
        let page = KERNEL_HEAP.virtual_allocate::<Size4K>(1).start;
        KERNEL_MEMORY_MAPPER.map(page, frame, PageFlags::device());
        page
    }

    fn map_device_pages(&self, frames: Range<Frame>) -> Range<Page> {
        let num_pages = Step::steps_between(&frames.start, &frames.end).unwrap();
        let pages = KERNEL_HEAP.virtual_allocate::<Size4K>(num_pages);
        for i in 0..num_pages {
            let frame = Step::forward(frames.start, i);
            let page = Step::forward(pages.start, i);
            KERNEL_MEMORY_MAPPER.map(page, frame, PageFlags::device());
        }
        pages
    }

    fn set_irq_handler(&self, irq: usize, handler: Box<dyn Fn() -> isize>) {
        crate::modules::INTERRUPT.set_irq_handler(irq, handler);
    }

    fn enable_irq(&self, irq: usize) {
        crate::modules::INTERRUPT.enable_irq(irq);
    }

    fn disable_irq(&self, irq: usize) {
        crate::modules::INTERRUPT.disable_irq(irq);
    }

    fn set_interrupt_controller(&self, controller: &'static dyn interrupt::InterruptController) {
        crate::modules::INTERRUPT.set_interrupt_controller(controller);
    }

    fn interrupt_controller(&self) -> &'static dyn interrupt::InterruptController {
        &*crate::modules::INTERRUPT
    }

    fn scheduler(&self) -> &'static dyn sched::Scheduler {
        &*crate::modules::SCHEDULER
    }

    fn num_cores(&self) -> usize {
        1
    }

    fn current_core(&self) -> usize {
        0
    }

    fn get_scheduler_state(&self, task: TaskId) -> &dyn Any {
        let task = SCHEDULER.get_task_by_id(task).unwrap();
        unsafe { &*(task.sched.as_ref() as *const dyn Any) }
    }

    unsafe fn return_to_user(&self, task: TaskId) -> ! {
        // Note: `task` must be dropped before calling `return_to_user`.
        let task = SCHEDULER.get_task_by_id(task).unwrap();
        let context_ptr = &task.context as *const <TargetArch as Arch>::Context;
        drop(task);
        (*context_ptr).return_to_user()
    }

    fn set_scheduler(&self, scheduler: &'static dyn sched::Scheduler) {
        SCHEDULER.set_scheduler(scheduler);
    }
    fn timer_controller(&self) -> &'static dyn interrupt::TimerController {
        &*crate::modules::TIMER
    }
    fn set_timer_controller(&self, timer: &'static dyn interrupt::TimerController) {
        crate::modules::TIMER.set_timer_controller(timer)
    }
}
