use crate::{
    memory::{
        kernel::{KERNEL_HEAP, KERNEL_MEMORY_MAPPER},
        page_table::PageFlags,
    },
    utils::page::*,
};
use device_tree::{DeviceTree, Node};

pub trait BootDriver {
    const COMPATIBLE: &'static str;
    fn init(&mut self, node: &Node);
    fn map_device_page(frame: Frame) -> Page {
        let page = KERNEL_HEAP.virtual_allocate::<Size4K>(1).start;
        KERNEL_MEMORY_MAPPER.map_fixed(page, frame, PageFlags::device());
        page
    }
}

impl<T: BootDriver> DynBootDriver for T {
    fn init(&self, dt: &DeviceTree) {
        dt.root.walk(&mut |node| match node.prop_str("compatible") {
            Ok(s) if s.split('\0').find(|x| *x == Self::COMPATIBLE).is_some() => {
                unsafe { &mut *(self as *const Self as *mut Self) }.init(node);
                true
            }
            _ => false,
        });
    }
}

pub trait DynBootDriver {
    fn init(&self, dt: &DeviceTree);
}

pub fn init(device_tree: &DeviceTree, drivers: &[&dyn DynBootDriver]) {
    for driver in drivers {
        driver.init(device_tree);
    }
}
