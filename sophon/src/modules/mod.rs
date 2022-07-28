use alloc::{borrow::ToOwned, boxed::Box, collections::BTreeMap, string::String, vec::Vec};
use core::alloc::GlobalAlloc;
use core::iter::Step;
use kernel_module::KernelServiceWrapper;
use kernel_module::ModuleCallHandler;
use memory::{
    address::Address,
    page::{Page, PageResource, Size4K},
};
use spin::{Lazy, Mutex};

use crate::memory::kernel::KERNEL_HEAP;

fn load_elf(elf_data: &[u8]) -> extern "C" fn(kernel_module::KernelServiceWrapper) -> usize {
    let entry = elf_loader::ELFLoader::load(elf_data, &mut |pages| {
        KERNEL_HEAP
            .acquire_pages::<Size4K>(Page::steps_between(&pages.start, &pages.end).unwrap())
            .unwrap()
    })
    .unwrap();
    log!("KM Entry: {:?}", entry);
    unsafe { core::mem::transmute(entry) }
}

struct KernelModule {
    name: String,
    service: Box<KernelService>,
    init: extern "C" fn(kernel_module::KernelServiceWrapper) -> usize,
    deinit: Option<extern "C" fn()>,
    call: Option<&'static dyn ModuleCallHandler>,
    elf: Vec<u8>,
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
                name: name.to_owned(),
                init,
                service,
                deinit: None,
                call: None,
                elf,
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
        log!("{}", s);
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
        log!("register module call");
        MODULES
            .lock()
            .get_mut(&self.0)
            .map(|module| {
                module.call = Some(handler);
            })
            .unwrap();
    }
}

pub fn module_call(module: &'static str, args: [usize; 4]) -> isize {
    log!("module call #{} {:x?}", module, args);
    let id = *MODULE_NAMES.lock().get(module).unwrap();
    MODULES
        .lock()
        .get(&id)
        .map(|module| {
            module
                .call
                .as_ref()
                .map(|call| call.handle(args))
                .unwrap_or(-1)
        })
        .unwrap_or(-1)
}
