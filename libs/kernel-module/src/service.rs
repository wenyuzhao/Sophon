use alloc::boxed::Box;
use core::alloc::Layout;
use core::any::Any;
use core::ops::{Deref, Range};
use device_tree::DeviceTree;
use interrupt::{InterruptController, TimerController};
use log::Logger;
use memory::address::Address;
use memory::page::{Frame, Page};
use proc::TaskId;
use sched::Scheduler;
use syscall::RawModuleRequest;
use testing::Tests;

pub trait KernelService: Send + Sync + 'static {
    // === Logging === //
    fn log(&self, s: &str);
    fn set_sys_logger(&self, logger: &'static dyn Logger);

    // === Testing === //
    fn register_tests(&self, tests: Tests);

    // === Module calls === //
    fn register_module_call_handler(&self, handler: &'static dyn super::ModuleCallHandler);
    fn module_call<'a>(&self, module: &str, request: RawModuleRequest<'a>) -> isize;

    // === Heap === //
    fn alloc(&self, layout: Layout) -> Option<Address>;
    fn dealloc(&self, address: Address, layout: Layout);

    // === Process === //
    /// Get process manager.
    fn process_manager(&self) -> &'static dyn proc::ProcessManager;
    /// Set process manager.
    fn set_process_manager(&self, process_manager: &'static dyn proc::ProcessManager);
    /// Kernel module panic handler.
    fn handle_panic(&self) -> !;
    /// Create memory state
    fn create_mm_state(&self) -> Box<dyn Any>;

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
    /// Return from kernel space to user space.
    unsafe fn return_to_user(&self, task: TaskId) -> !;
    /// Get the scheduler.
    fn scheduler(&self) -> &'static dyn Scheduler;
    /// Set the scheduler.
    fn set_scheduler(&self, scheduler: &'static dyn Scheduler);
    /// Arch-dependent task context
    fn create_task_context(&self) -> Box<dyn Any>;
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
