use alloc::boxed::Box;
use core::alloc::Layout;
use core::ops::{Deref, Range};
use device_tree::DeviceTree;
use interrupt::InterruptController;
use log::Logger;
use memory::address::Address;
use memory::page::{Frame, Page};
use proc::{ProcId, TaskId};
use syscall::RawModuleRequest;
use testing::Tests;

pub trait KernelService: Send + Sync + 'static {
    // Logging
    fn log(&self, s: &str);
    fn set_sys_logger(&self, logger: &'static dyn Logger);
    // Testing
    fn register_tests(&self, tests: Tests);
    // Module calls
    fn register_module_call_handler(&self, handler: &'static dyn super::ModuleCallHandler);
    fn module_call<'a>(&self, module: &str, request: RawModuleRequest<'a>) -> isize;
    // Heap
    fn alloc(&self, layout: Layout) -> Option<Address>;
    fn dealloc(&self, address: Address, layout: Layout);
    // Process
    fn current_process(&self) -> Option<ProcId>;
    fn current_task(&self) -> Option<TaskId>;
    fn handle_panic(&self) -> !;
    // Devices
    fn set_interrupt_controller(&self, controller: &'static dyn InterruptController);
    fn get_device_tree(&self) -> Option<&'static DeviceTree<'static, 'static>>;
    fn map_device_page(&self, frame: Frame) -> Page;
    fn map_device_pages(&self, frames: Range<Frame>) -> Range<Page>;
    fn set_irq_handler(&self, irq: usize, handler: Box<dyn Fn() -> isize>);
    fn enable_irq(&self, irq: usize);
    fn disable_irq(&self, irq: usize);
    // Scheduler
    fn interrupt_controller(&self) -> &'static dyn InterruptController;
    fn schedule(&self) -> !;
    fn num_cores(&self) -> usize;
    fn current_core(&self) -> usize;
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct KernelServiceWrapper([usize; 2]);

impl KernelServiceWrapper {
    pub fn get_service(self) -> &'static dyn KernelService {
        unsafe { core::mem::transmute(self) }
    }
    pub fn from_service(service: &'static dyn KernelService) -> Self {
        unsafe { core::mem::transmute(service) }
    }
}

impl Deref for KernelServiceWrapper {
    type Target = dyn KernelService;

    fn deref(&self) -> &'static Self::Target {
        self.get_service()
    }
}
