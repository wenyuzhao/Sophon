use super::raw_module_call;
use super::MODULES;
use crate::arch::{Arch, TargetArch};
use crate::memory::kernel::KERNEL_HEAP;
use crate::memory::kernel::KERNEL_MEMORY_MAPPER;
use crate::task::proc::PROCESS_MANAGER;
use crate::task::sched::SCHEDULER;
use crate::task::sync::SysMonitor;
use crate::utils::testing::Tests;
use alloc::boxed::Box;
use alloc::sync::Arc;
use core::alloc::GlobalAlloc;
use core::iter::Step;
use core::ops::Range;
use device_tree::DeviceTree;
use kernel_module::ModuleCallHandler;
use klib::proc::Process;
use memory::page::Frame;
use memory::page_table::PageFlags;
use memory::{
    address::Address,
    page::{Page, Size4K},
};
use vfs::ramfs::RamFS;

pub struct KernelService(pub usize);

impl kernel_module::KernelService for KernelService {
    fn log(&self, s: &str) {
        // trace!("KernelService::log: {}", s);
        print!("{}", s);
    }

    fn create_monitor(&self) -> alloc::boxed::Box<dyn kernel_module::monitor::SysMonitor> {
        let handle = SysMonitor::new();
        struct Wrapper {
            handle: SysMonitor,
        }
        impl kernel_module::monitor::SysMonitor for Wrapper {
            fn lock(&self) {
                self.handle.lock();
            }
            fn unlock(&self) {
                self.handle.unlock();
            }
            fn notify_all(&self) {
                self.handle.notify_all();
            }
            fn wait(&self) {
                self.handle.wait();
            }
        }
        Box::new(Wrapper { handle })
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

    fn handle_panic(&self) -> ! {
        if cfg!(sophon_test) {
            TargetArch::halt(-1)
        }
        syscall::exit();
    }

    fn vfs(&self) -> &'static dyn vfs::VFSManager {
        &*crate::modules::VFS
    }

    #[allow(invalid_reference_casting)]
    fn set_vfs_manager(&self, vfs_manager: &'static dyn vfs::VFSManager) {
        crate::modules::VFS.set_instance(vfs_manager);
        let initfs = *crate::INIT_FS.get().unwrap() as *const RamFS;
        let ptr = initfs as *mut RamFS;
        vfs_manager.init(unsafe { &mut *ptr });
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

    fn timer_tick(&self) -> ! {
        SCHEDULER.timer_tick()
    }

    fn current_pid(&self) -> klib::proc::PID {
        PROCESS_MANAGER.current_proc_id().unwrap()
    }

    fn current_proc(&self) -> Option<Arc<Process>> {
        PROCESS_MANAGER.current_proc()
    }
}
