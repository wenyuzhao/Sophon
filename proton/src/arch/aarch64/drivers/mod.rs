mod bcm2711_gpio;
mod gic;
mod uart;

use self::bcm2711_gpio::GPIO;
use self::gic::GIC;
use self::uart::UART;
use device_tree::DeviceTree;

pub fn init(device_tree: &DeviceTree) {
    crate::boot_driver::init(device_tree, &[&GPIO, &UART, &GIC]);
}
