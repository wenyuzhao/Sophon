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
    fn init_with_device_tree(&self, dt: &DeviceTree) {
        dt.root.walk(&mut |node| match node.prop_str("compatible") {
            Ok(s) if s.split('\0').find(|x| *x == Self::COMPATIBLE).is_some() => {
                unsafe { &mut *(self as *const Self as *mut Self) }.init(node);
                true
            }
            _ => false,
        });
    }
    fn map_device_page(frame: Frame) -> Page {
        let page = KERNEL_HEAP.virtual_allocate::<Size4K>(1).start;
        KERNEL_MEMORY_MAPPER.map_fixed(page, frame, PageFlags::device());
        page
    }
}

pub trait DynBootDriver {}

pub trait InterruptController {}

// pub struct BootDriverManager {
//     drivers: Vec<&'static dyn Any>,
// }

// impl BootDriverManager {
//     pub fn boot() -> Self {}
// }
