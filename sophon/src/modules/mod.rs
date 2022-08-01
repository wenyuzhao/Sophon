use alloc::{borrow::ToOwned, boxed::Box, collections::BTreeMap, string::String, vec::Vec};
use core::alloc::GlobalAlloc;
use core::iter::Step;
use core::mem;
use devtree::DeviceTree;
use kernel_module::KernelServiceWrapper;
use kernel_module::ModuleCallHandler;
use memory::page::Frame;
use memory::page_table::PageFlags;
use memory::page_table::PageFlagsExt;
use memory::{
    address::Address,
    page::{Page, PageResource, Size4K},
};
use proc::ProcId;
use spin::{Lazy, Mutex};
use syscall::RawModuleRequest;
use vfs::ramfs::RamFS;

use crate::arch::{Arch, TargetArch};
use crate::memory::kernel::KERNEL_HEAP;
use crate::memory::kernel::KERNEL_MEMORY_MAPPER;
use crate::task::scheduler::mutex::SysMonitor;
use crate::task::scheduler::AbstractScheduler;
use crate::task::scheduler::SCHEDULER;
use crate::task::Proc;
use crate::task::Task;

fn load_elf(elf_data: &[u8]) -> extern "C" fn(kernel_module::KernelServiceWrapper) -> usize {
    let entry = elf_loader::ELFLoader::load(elf_data, &mut |pages| {
        let range = KERNEL_HEAP
            .acquire_pages::<Size4K>(Page::steps_between(&pages.start, &pages.end).unwrap())
            .unwrap();
        // log!("code: {:?}", range);
        range
    })
    .unwrap();
    unsafe { core::mem::transmute(entry) }
}

struct KernelModule {
    _name: String,
    _service: Box<KernelService>,
    _deinit: Option<extern "C" fn()>,
    call: Option<&'static dyn ModuleCallHandler>,
    _elf: Vec<u8>,
}

static MODULES: Lazy<Mutex<BTreeMap<usize, KernelModule>>> = Lazy::new(Default::default);
static MODULE_NAMES: Lazy<Mutex<BTreeMap<String, usize>>> = Lazy::new(Default::default);

pub fn register(name: &str, elf: Vec<u8>) {
    let (init, service_ptr) = {
        let mut modules = MODULES.lock();
        let mut names = MODULE_NAMES.lock();
        if names.contains_key(name) {
            return;
        }
        let init = load_elf(&elf);
        let id = modules.len();
        let service = box KernelService(id);
        let service_ptr = service.as_ref() as *const KernelService;
        modules.insert(
            id,
            KernelModule {
                _name: name.to_owned(),
                _service: service,
                _deinit: None,
                call: None,
                _elf: elf,
            },
        );
        names.insert(name.to_owned(), id);
        (init, service_ptr)
    };
    init(KernelServiceWrapper::from_service(unsafe { &*service_ptr }));
}

pub struct KernelService(usize);

impl kernel_module::KernelService for KernelService {
    fn log(&self, s: &str) {
        print!("{}", s);
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
        module_call(module, true, request.as_buf())
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

    fn schedule(&self) -> ! {
        TargetArch::interrupt().notify_end_of_interrupt();
        SCHEDULER.timer_tick();
        unreachable!()
    }

    fn new_monitor(&self) -> mutex::Monitor {
        mutex::Monitor::new(SysMonitor::new())
    }
}

pub fn module_call(module: &str, privileged: bool, args: [usize; 4]) -> isize {
    // log!("module call #{} {:x?}", module, args);
    let id = *MODULE_NAMES.lock().get(module).unwrap();
    MODULES
        .lock()
        .get(&id)
        .map(|module| {
            module
                .call
                .as_ref()
                .map(|call| call.handle(privileged, RawModuleRequest::from_buf(args)))
                .unwrap_or(-1)
        })
        .unwrap_or(-1)
}

pub fn init_vfs(ramfs: &'static RamFS) {
    module_call("vfs", true, [0, unsafe { mem::transmute(ramfs) }, 0, 0]);
}
