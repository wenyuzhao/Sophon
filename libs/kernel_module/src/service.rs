use core::alloc::Layout;
use core::ops::Deref;
use memory::address::Address;

pub trait KernelService: Send + Sync + 'static {
    fn log(&self, s: &str);
    fn alloc(&self, layout: Layout) -> Option<Address>;
    fn dealloc(&self, address: Address, layout: Layout);
    fn register_module_call_handler(&self, handler: &'static dyn super::ModuleCallHandler);
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
