use device_tree::{DeviceTree, Node};

pub trait BootDriver {
    const COMPATIBLE: &'static str;
    fn init(&mut self, node: &Node);
    fn init_with_device_tree(&self, dt: &DeviceTree) {
        dt.root.walk(&mut |node| match node.prop_str("compatible") {
            Ok(s) if s == Self::COMPATIBLE => {
                unsafe { &mut *(self as *const Self as *mut Self) }.init(node);
                true
            }
            _ => false,
        });
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
