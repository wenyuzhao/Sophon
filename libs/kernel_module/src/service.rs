use alloc::boxed::Box;
use core::alloc::Layout;
use core::ops::Deref;
use devtree::DeviceTree;
use interrupt::InterruptController;
use log::Logger;
use memory::address::Address;
use memory::page::{Frame, Page};
use mutex::Monitor;
use proc::{ProcId, TaskId};
use syscall::RawModuleRequest;

pub trait KernelService: Send + Sync + 'static {
    // Logging
    fn log(&self, s: &str);
    fn set_sys_logger(&self, logger: &'static dyn Logger);
    // Module calls
    fn register_module_call_handler(&self, handler: &'static dyn super::ModuleCallHandler);
    fn module_call<'a>(&self, module: &str, request: RawModuleRequest<'a>) -> isize;
    // Heap
    fn alloc(&self, layout: Layout) -> Option<Address>;
    fn dealloc(&self, address: Address, layout: Layout);
    // Process
    fn current_process(&self) -> Option<ProcId>;
    fn current_task(&self) -> Option<TaskId>;
    // Devices
    fn set_interrupt_controller(&self, controller: &'static dyn InterruptController);
    fn get_device_tree(&self) -> Option<&'static DeviceTree<'static, 'static>>;
    fn map_device_page(&self, frame: Frame) -> Page;
    fn set_irq_handler(&self, irq: usize, handler: Box<dyn Fn() -> isize>);
    fn enable_irq(&self, irq: usize);
    fn disable_irq(&self, irq: usize);
    // Scheduler
    fn schedule(&self) -> !;
    fn new_monitor(&self) -> Monitor;
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
