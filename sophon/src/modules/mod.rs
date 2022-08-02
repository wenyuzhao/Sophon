use alloc::{borrow::ToOwned, boxed::Box, collections::BTreeMap, string::String, vec::Vec};
use core::iter::Step;
use kernel_module::KernelServiceWrapper;
use kernel_module::ModuleCallHandler;
use memory::page::{Page, PageResource, Size4K};
use spin::{Lazy, Mutex};
use syscall::RawModuleRequest;
use vfs::ramfs::RamFS;
use vfs::VFSRequest;

use crate::memory::kernel::KERNEL_HEAP;

use self::services::KernelService;

mod services;

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

pub fn raw_module_call(module: &str, privileged: bool, args: [usize; 4]) -> isize {
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

pub fn module_call<'a>(
    module: &str,
    privileged: bool,
    request: &'a impl syscall::ModuleRequest<'a>,
) -> isize {
    raw_module_call(module, privileged, request.as_raw().as_buf())
}

pub fn init_vfs(ramfs: *mut RamFS) {
    module_call("vfs", true, &VFSRequest::Init(unsafe { &mut *ramfs }));
}
