use super::raw_module_call;
use super::MODULES;
use super::PROCESS_MANAGER;
use crate::arch::ArchContext;
use crate::arch::{Arch, TargetArch};
use crate::memory::kernel::KERNEL_HEAP;
use crate::memory::kernel::KERNEL_MEMORY_MAPPER;
use crate::modules::SCHEDULER;
use crate::task::MMState;
use crate::utils::testing::Tests;
use alloc::boxed::Box;
use core::alloc::GlobalAlloc;
use core::any::Any;
use core::iter::Step;
use core::ops::Range;
use device_tree::DeviceTree;
use kernel_module::ModuleCallHandler;
use memory::page::Frame;
use memory::page_table::PageFlags;
use memory::{
    address::Address,
    page::{Page, Size4K},
};
use proc::TaskId;

pub struct KernelService(pub usize);

impl kernel_module::KernelService for KernelService {
    fn log(&self, s: &str) {
        print!("{}", s);
    }

    fn set_sys_logger(&self, write: *mut dyn core::fmt::Write) {
        crate::utils::print::init(unsafe { &mut *write });
    }

    fn register_tests(&self, tests: Tests) {
        crate::utils::testing::register_kernel_tests(tests);
    }

    fn register_module_call_handler(&self, handler: &'static dyn ModuleCallHandler) {
        MODULES.write()[self.0].as_mut().map(|m| {
            m.call = Some(handler);
        });
    }

    fn module_call<'a>(&self, module: &str, request: syscall::RawModuleRequest<'a>) -> isize {
        raw_module_call(module, true, request.as_buf())
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

    fn process_manager(&self) -> &'static dyn proc::ProcessManager {
        &*crate::modules::PROCESS_MANAGER
    }

    fn set_process_manager(&self, process_manager: &'static dyn proc::ProcessManager) {
        crate::modules::PROCESS_MANAGER.set_instance(process_manager);
    }

    fn handle_panic(&self) -> ! {
        if cfg!(sophon_test) {
            TargetArch::halt(-1)
        }
        syscall::exit();
    }

    fn create_mm_state(&self) -> Box<dyn Any> {
        MMState::new()
    }

    fn vfs(&self) -> &'static dyn vfs::VFSManager {
        &*crate::modules::VFS
    }

    fn set_vfs_manager(&self, vfs_manager: &'static dyn vfs::VFSManager) {
        crate::modules::VFS.set_instance(vfs_manager);
        vfs_manager.init(unsafe { &mut *crate::INIT_FS.unwrap() });
    }

    #[allow(static_mut_refs)]
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

    fn interrupt_controller(&self) -> &'static dyn interrupt::InterruptController {
        &*crate::modules::INTERRUPT
    }

    fn set_interrupt_controller(&self, controller: &'static dyn interrupt::InterruptController) {
        crate::modules::INTERRUPT.set_instance(controller);
    }

    fn timer_controller(&self) -> &'static dyn interrupt::TimerController {
        &*crate::modules::TIMER
    }

    fn set_timer_controller(&self, timer: &'static dyn interrupt::TimerController) {
        crate::modules::TIMER.set_instance(timer)
    }

    fn num_cores(&self) -> usize {
        1
    }

    fn current_core(&self) -> usize {
        0
    }

    unsafe fn return_to_user(&self, task: TaskId) -> ! {
        // Note: `task` must be dropped before calling `return_to_user`.
        let task = PROCESS_MANAGER.get_task_by_id(task).unwrap();
        let context_ptr = {
            task.context
                .downcast_ref_unchecked::<<TargetArch as Arch>::Context>()
                as *const <TargetArch as Arch>::Context
        };
        drop(task);
        (*context_ptr).return_to_user()
    }

    fn scheduler(&self) -> &'static dyn sched::Scheduler {
        &*crate::modules::SCHEDULER
    }

    fn set_scheduler(&self, scheduler: &'static dyn sched::Scheduler) {
        SCHEDULER.set_instance(scheduler);
    }

    fn create_task_context(&self) -> Box<dyn Any> {
        Box::new(<TargetArch as Arch>::Context::new(
            crate::task::entry as _,
            0 as _,
        ))
    }
}
