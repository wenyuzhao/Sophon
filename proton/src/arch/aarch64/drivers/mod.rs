pub mod bcm2711_gpio;
pub mod gic;
pub mod uart;

use self::bcm2711_gpio::GPIO;
use self::gic::GIC;
use self::uart::UART;
use fdt::Fdt;

pub unsafe fn init(device_tree: &Fdt) {
    crate::boot_driver::init(device_tree, &mut [&mut GPIO, &mut UART, &mut GIC]);
}
