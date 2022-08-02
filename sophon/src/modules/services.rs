use alloc::boxed::Box;
use core::alloc::GlobalAlloc;
use devtree::DeviceTree;
use kernel_module::ModuleCallHandler;
use log::Logger;
use memory::page::Frame;
use memory::page_table::PageFlags;
use memory::page_table::PageFlagsExt;
use memory::{
    address::Address,
    page::{Page, Size4K},
};
use proc::ProcId;

use crate::arch::{Arch, TargetArch};
use crate::memory::kernel::KERNEL_HEAP;
use crate::memory::kernel::KERNEL_MEMORY_MAPPER;
use crate::task::scheduler::monitor::SysMonitor;
use crate::task::scheduler::AbstractScheduler;
use crate::task::scheduler::SCHEDULER;
use crate::task::Proc;
use crate::task::Task;

use super::raw_module_call;
use super::MODULES;
pub struct KernelService(pub usize);

impl kernel_module::KernelService for KernelService {
    fn log(&self, s: &str) {
        print!("{}", s);
    }

    fn set_sys_logger(&self, logger: &'static dyn Logger) {
        log::init(logger)
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
        MODULES
            .lock()
            .get_mut(&self.0)
            .map(|module| {
                module.call = Some(handler);
            })
            .unwrap();
    }

    fn module_call<'a>(&self, module: &str, request: syscall::RawModuleRequest<'a>) -> isize {
        raw_module_call(module, true, request.as_buf())
    }

    fn current_process(&self) -> Option<ProcId> {
        Some(Proc::current().id)
    }

    fn current_task(&self) -> Option<proc::TaskId> {
        Some(Task::current().id)
    }

    fn get_device_tree(&self) -> Option<&'static DeviceTree<'static, 'static>> {
        unsafe { crate::DEV_TREE.as_ref() }
    }

    fn map_device_page(&self, frame: Frame) -> Page {
        let page = KERNEL_HEAP.virtual_allocate::<Size4K>(1).start;
        KERNEL_MEMORY_MAPPER.map(page, frame, PageFlags::device());
        page
    }

    fn set_irq_handler(&self, irq: usize, handler: Box<dyn Fn() -> isize>) {
        TargetArch::interrupt().set_irq_handler(irq, handler);
    }

    fn enable_irq(&self, irq: usize) {
        TargetArch::interrupt().enable_irq(irq);
    }

    fn disable_irq(&self, irq: usize) {
        TargetArch::interrupt().disable_irq(irq);
    }

    fn set_interrupt_controller(&self, controller: &'static dyn interrupt::InterruptController) {
        TargetArch::set_interrupt_controller(controller);
    }

    fn schedule(&self) -> ! {
        TargetArch::interrupt().notify_end_of_interrupt();
        SCHEDULER.timer_tick();
        unreachable!()
    }

    fn new_monitor(&self) -> mutex::Monitor {
        mutex::Monitor::new(SysMonitor::new())
    }
}
