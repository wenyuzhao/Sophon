use core::alloc::Layout;
use core::ops::Deref;
use memory::address::Address;
use memory::page::{Frame, Page};
use proc::ProcId;

pub trait KernelService: Send + Sync + 'static {
    // Logging
    fn log(&self, s: &str);
    // Module calls
    fn register_module_call_handler(&self, handler: &'static dyn super::ModuleCallHandler);
    // Heap
    fn alloc(&self, layout: Layout) -> Option<Address>;
    fn dealloc(&self, address: Address, layout: Layout);
    // Process
    fn current_process(&self) -> Option<ProcId>;
    // Devices
    fn get_device_tree(&self) -> Option<fdt::Fdt<'static>>;
    fn map_device_page(&self, frame: Frame) -> Page;
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
