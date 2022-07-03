use crate::{boot_driver::BootDriver, utils::volatile::Volatile};
use core::arch::asm;
use fdt::node::FdtNode;
use memory::{
    address::{Address, P},
    page::Frame,
};

#[repr(C)]
pub struct GPIORegisters {
    pub gpfsel0: Volatile<u32>,   // 0x0
    pub gpfsel1: Volatile<u32>,   // 0x04
    pub gpfsel2: Volatile<u32>,   // 0x08
    pub gpfsel3: Volatile<u32>,   // 0x0c
    pub gpfsel4: Volatile<u32>,   // 0x10
    pub gpfsel5: Volatile<u32>,   // 0x14
    _0: [u8; 4],                  // 0x18
    pub gpset0: Volatile<u32>,    // 0x1c
    pub gpset1: Volatile<u32>,    // 0x20
    _1: [u8; 4],                  // 0x24
    pub gpclr0: Volatile<u32>,    // 0x28
    pub gpclr1: Volatile<u32>,    // 0x2c
    _2: [u8; 4],                  // 0x30
    pub gplev0: Volatile<u32>,    // 0x34
    pub gplev1: Volatile<u32>,    // 0x38
    _3: [u8; 4],                  // 0x3c
    pub gpeds0: Volatile<u32>,    // 0x40
    pub gpeds1: Volatile<u32>,    // 0x44
    _4: [u8; 4],                  // 0x48
    pub gpren0: Volatile<u32>,    // 0x4c
    pub gpren1: Volatile<u32>,    // 0x50
    _5: [u8; 4],                  // 0x54
    pub gpfen0: Volatile<u32>,    // 0x58
    pub gpfen1: Volatile<u32>,    // 0x5c
    _6: [u8; 4],                  // 0x60
    pub gphen0: Volatile<u32>,    // 0x64
    pub gphen1: Volatile<u32>,    // 0x68
    _7: [u8; 4],                  // 0x6c
    pub gplen0: Volatile<u32>,    // 0x70
    pub gplen1: Volatile<u32>,    // 0x74
    _8: [u8; 4],                  // 0x78
    pub gparen0: Volatile<u32>,   // 0x7c
    pub gparen1: Volatile<u32>,   // 0x80
    _9: [u8; 4],                  // 0x84
    pub gpafen0: Volatile<u32>,   // 0x88
    pub gpafen1: Volatile<u32>,   // 0x8c
    _10: [u8; 4],                 // 0x90
    pub gppud: Volatile<u32>,     // 0x94
    pub gppudclk0: Volatile<u32>, // 0x98
    pub gppudclk1: Volatile<u32>, // 0x9c
}

pub struct GPIO {
    gpio: Option<*mut GPIORegisters>,
}

unsafe impl Send for GPIO {}
unsafe impl Sync for GPIO {}

impl GPIO {
    const fn new() -> Self {
        Self { gpio: None }
    }

    fn gpio(&self) -> &mut GPIORegisters {
        unsafe { &mut *self.gpio.unwrap() }
    }

    #[inline(never)]
    fn wait_cycles(&self, n: usize) {
        for _ in 0..n {
            unsafe {
                asm!("nop");
            }
        }
    }

    fn init_gpio(&self) {
        let gpio = self.gpio();
        gpio.gpfsel1.set((0b100 << 12) | (0b100 << 15));
        gpio.gppud.set(0);
        self.wait_cycles(150);
        gpio.gppudclk0.set((1 << 14) | (1 << 15));
        self.wait_cycles(150);
        gpio.gppudclk0.set(0);
    }
}

pub static mut GPIO: GPIO = GPIO::new();

impl BootDriver for GPIO {
    const COMPATIBLE: &'static [&'static str] = &["brcm,bcm2711-gpio"];
    fn init(&mut self, node: &FdtNode, parent: Option<&FdtNode>) {
        let mut gpio_frame =
            Address::<P>::new(node.reg().unwrap().next().unwrap().starting_address as usize);
        gpio_frame = Self::translate_address(gpio_frame, parent.unwrap());
        let gpio_page = Self::map_device_page(Frame::new(gpio_frame));
        self.gpio = Some(gpio_page.start().as_mut_ptr());
        self.init_gpio();
    }
}
