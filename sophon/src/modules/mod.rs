use core::{iter::Step, ptr};

use alloc::{borrow::ToOwned, collections::BTreeMap, string::String, vec::Vec};
use kernel_module::KernelServiceWrapper;
use memory::{
    address::{Address, V},
    page::{Page, PageResource, PageSize, Size2M, Size4K},
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
    init: extern "C" fn(kernel_module::KernelServiceWrapper) -> usize,
    deinit: Option<extern "C" fn()>,
    elf: Vec<u8>,
}

static MODULES: Lazy<Mutex<BTreeMap<String, KernelModule>>> = Lazy::new(Default::default);

pub fn register(name: &str, elf: Vec<u8>) {
    let init = {
        let mut modules = MODULES.lock();
        if modules.contains_key(name) {
            return;
        }
        let init = load_elf(&elf);
        modules.insert(
            name.to_owned(),
            KernelModule {
                name: name.to_owned(),
                init,
                deinit: None,
                elf,
            },
        );
        init
    };
    init(KernelServiceWrapper::from_service(&KERNEL_SERVICE));
}

pub struct KernelService;

impl kernel_module::KernelService for KernelService {
    fn log(&self, s: &str) {
        log!("{}", s);
    }
}

static KERNEL_SERVICE: KernelService = KernelService;
