pub mod bcm2711_gpio;
pub mod gic;

use self::bcm2711_gpio::GPIO;
use self::gic::GIC;
use devtree::DeviceTree;

pub unsafe fn init(device_tree: &'static DeviceTree<'static, 'static>) {
    crate::boot_driver::init(device_tree, &mut [&mut GPIO, &mut GIC]);
}
