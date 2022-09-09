use alloc::{borrow::ToOwned, boxed::Box, collections::BTreeMap, string::String, vec::Vec};
use core::iter::Step;
use kernel_module::KernelServiceWrapper;
use kernel_module::ModuleCallHandler;
use memory::page::{Page, PageResource, Size4K};
use spin::RwLock;
use syscall::RawModuleRequest;

use crate::memory::kernel::KERNEL_HEAP;

use self::services::KernelService;

mod named_modules;
mod services;

pub use named_modules::{INTERRUPT, PROCESS_MANAGER, SCHEDULER, TIMER, VFS};

struct KernelModule {
    _name: String,
    _service: Box<KernelService>,
    _deinit: Option<extern "C" fn()>,
    call: Option<&'static dyn ModuleCallHandler>,
    _elf: Vec<u8>,
}

const MAX_MODULES: usize = 256;
static MODULES: RwLock<[Option<Box<KernelModule>>; MAX_MODULES]> = {
    const UNINIT: Option<Box<KernelModule>> = None;
    RwLock::new([UNINIT; MAX_MODULES])
};
static MODULE_NAMES: RwLock<BTreeMap<String, usize>> = RwLock::new(BTreeMap::new());

fn load_elf(
    elf_data: &[u8],
) -> (
    extern "C" fn(kernel_module::KernelServiceWrapper) -> usize,
    &[extern "C" fn()],
) {
    let entry = elf_loader::ELFLoader::load(elf_data, &mut |pages| {
        let range = KERNEL_HEAP
            .acquire_pages::<Size4K>(Page::steps_between(&pages.start, &pages.end).unwrap())
            .unwrap();
        // log!("code: {:?}", range);
        range
    })
    .unwrap();
    let init_array = unsafe { core::mem::transmute(entry.init_array) };
    let entry = unsafe { core::mem::transmute(entry.entry) };
    (entry, init_array)
}

pub fn register(name: &str, elf: Vec<u8>) {
    let (start, service_ptr) = {
        let mut names = MODULE_NAMES.write();
        let mut modules = MODULES.write();
        if names.contains_key(name) {
            return;
        }
        let id = names.len();
        let (start, init_array) = load_elf(&elf);
        let service = box KernelService(id);
        let service_ptr = service.as_ref() as *const KernelService;
        for init in init_array {
            init()
        }
        modules[id] = Some(box KernelModule {
            _name: name.to_owned(),
            _service: service,
            _deinit: None,
            call: None,
            _elf: elf,
        });
        names.insert(name.to_owned(), id);
        (start, service_ptr)
    };
    start(KernelServiceWrapper::from_service(unsafe { &*service_ptr }));
}

pub fn raw_module_call(module: &str, privileged: bool, args: [usize; 4]) -> isize {
    // log!("module call #{} {:x?}", module, args);
    let _guard = ::interrupt::uninterruptible();
    let id = *MODULE_NAMES.read().get(module).unwrap();
    let modules_ptr = MODULES.read()[id]
        .as_ref()
        .map(|m| m.as_ref() as *const KernelModule);
    if let Some(modules_ptr) = modules_ptr {
        let m = unsafe { &*modules_ptr };
        m.call
            .as_ref()
            .map(|call| call.handle(privileged, RawModuleRequest::from_buf(args)))
            .unwrap_or(-1)
    } else {
        -1
    }
}

pub fn module_call<'a>(
    module: &str,
    privileged: bool,
    request: &'a impl syscall::ModuleRequest<'a>,
) -> isize {
    raw_module_call(module, privileged, request.as_raw().as_buf())
}
