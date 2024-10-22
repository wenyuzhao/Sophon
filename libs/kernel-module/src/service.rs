use alloc::sync::Arc;
use core::alloc::Layout;
use core::ops::{Deref, Range};
use device_tree::DeviceTree;
use interrupt::{InterruptController, TimerController};
use klib::proc::{Process, PID};
use memory::address::Address;
use memory::page::{Frame, Page};
use syscall::RawModuleRequest;
use testing::Tests;

pub trait KernelService: Send + Sync + 'static {
    // === Logging === //
    fn log(&self, s: &str);
    fn set_sys_logger(&self, write: *mut dyn core::fmt::Write);

    // === Testing === //
    fn register_tests(&self, tests: Tests);

    // === Module calls === //
    fn register_module_call_handler(&self, handler: &'static dyn super::ModuleCallHandler);
    fn module_call<'a>(&self, module: &str, request: RawModuleRequest<'a>) -> isize;

    // === Heap === //
    fn alloc(&self, layout: Layout) -> Option<Address>;
    fn dealloc(&self, address: Address, layout: Layout);

    /// Kernel module panic handler.
    fn handle_panic(&self) -> !;

    // === VFS === //
    /// Get VFS manager.
    fn vfs(&self) -> &'static dyn vfs::VFSManager;
    /// Set VFS manager.
    fn set_vfs_manager(&self, vfs_manager: &'static dyn vfs::VFSManager);

    // === Devices === //
    fn get_device_tree(&self) -> Option<&'static DeviceTree<'static, 'static>>;
    fn map_device_page(&self, frame: Frame) -> Page;
    fn map_device_pages(&self, frames: Range<Frame>) -> Range<Page>;

    // === Interrupt and Timer === //
    /// Get interrupt controller.
    fn interrupt_controller(&self) -> &'static dyn InterruptController;
    /// Set interrupt controller.
    fn set_interrupt_controller(&self, controller: &'static dyn InterruptController);
    /// Get timer controller.
    fn timer_controller(&self) -> &'static dyn TimerController;
    /// Set timer controller.
    fn set_timer_controller(&self, timer: &'static dyn TimerController);

    // === Scheduler === //
    /// Get number of logical cores.
    fn num_cores(&self) -> usize;
    /// Get the current core.
    /// Returning `0` means its a BSP.
    fn current_core(&self) -> usize;

    fn timer_tick(&self) -> !;
    fn current_pid(&self) -> PID;
    fn current_proc(&self) -> Option<Arc<Process>>;
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct KernelServiceWrapper([usize; 2]);

impl KernelServiceWrapper {
    #[inline(always)]
    pub fn get_service(self) -> &'static dyn KernelService {
        unsafe { core::mem::transmute(self) }
    }

    pub fn from_service(service: &'static dyn KernelService) -> Self {
        unsafe { core::mem::transmute(service) }
    }
}

impl Deref for KernelServiceWrapper {
    type Target = dyn KernelService;

    #[inline(always)]
    fn deref(&self) -> &'static Self::Target {
        self.get_service()
    }
}
