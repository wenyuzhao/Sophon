use crate::memory::kernel::{KERNEL_HEAP, KERNEL_MEMORY_MAPPER};
use devtree::{DeviceTree, Node};
use memory::page::*;
use memory::page_table::{PageFlags, PageFlagsExt};

pub trait BootDriver {
    const COMPATIBLE: &'static [&'static str];
    fn init(&mut self, node: &Node);
    fn map_device_page(frame: Frame) -> Page {
        let page = KERNEL_HEAP.virtual_allocate::<Size4K>(1).start;
        KERNEL_MEMORY_MAPPER.map(page, frame, PageFlags::device());
        page
    }
}

impl<T: BootDriver> DynBootDriver for T {
    fn init(&mut self, devtree: &DeviceTree) {
        for s in Self::COMPATIBLE {
            if let Some(node) = devtree.compatible(s) {
                self.init(&node);
                return;
            }
        }
    }
}

pub trait DynBootDriver {
    fn init(&mut self, devtree: &DeviceTree);
}

pub fn init(device_tree: &DeviceTree, drivers: &mut [&mut dyn DynBootDriver]) {
    for driver in drivers {
        driver.init(&device_tree);
    }
}
