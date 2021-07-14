use crate::{
    memory::{
        kernel::{KERNEL_HEAP, KERNEL_MEMORY_MAPPER},
        page_table::PageFlags,
    },
    utils::page::*,
};
use fdt::{node::FdtNode, Fdt};

pub trait BootDriver {
    const COMPATIBLE: &'static [&'static str];
    fn init(&mut self, node: &FdtNode);
    fn map_device_page(frame: Frame) -> Page {
        let page = KERNEL_HEAP.virtual_allocate::<Size4K>(1).start;
        KERNEL_MEMORY_MAPPER.map_fixed(page, frame, PageFlags::device());
        page
    }
}

impl<T: BootDriver> DynBootDriver for T {
    fn init(&mut self, fdt: &Fdt) {
        if let Some(node) = fdt.find_compatible(Self::COMPATIBLE) {
            self.init(&node)
        }
    }
}

pub trait DynBootDriver {
    fn init(&mut self, fdt: &Fdt);
}

pub fn init(device_tree: &Fdt, drivers: &mut [&mut dyn DynBootDriver]) {
    for driver in drivers {
        driver.init(&device_tree);
    }
}
