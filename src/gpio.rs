

#[cfg(feature="raspi3")]
pub const PERIPHERAL_BASE: usize = 0x3F000000;
#[cfg(feature="raspi4")]
pub const PERIPHERAL_BASE: usize = 0xFE000000;

pub const GPIO_BASE: usize = PERIPHERAL_BASE + 0x200000;

pub const GPFSEL0:   *mut u32 = (GPIO_BASE + 0x00) as _;
pub const GPFSEL1:   *mut u32 = (GPIO_BASE + 0x04) as _;
pub const GPFSEL2:   *mut u32 = (GPIO_BASE + 0x08) as _;
pub const GPFSEL3:   *mut u32 = (GPIO_BASE + 0x0C) as _;
pub const GPFSEL4:   *mut u32 = (GPIO_BASE + 0x10) as _;
pub const GPFSEL5:   *mut u32 = (GPIO_BASE + 0x14) as _;
pub const GPSET0:    *mut u32 = (GPIO_BASE + 0x1C) as _;
pub const GPSET1:    *mut u32 = (GPIO_BASE + 0x20) as _;
pub const GPCLR0:    *mut u32 = (GPIO_BASE + 0x28) as _;
pub const GPLEV0:    *mut u32 = (GPIO_BASE + 0x34) as _;
pub const GPLEV1:    *mut u32 = (GPIO_BASE + 0x38) as _;
pub const GPEDS0:    *mut u32 = (GPIO_BASE + 0x40) as _;
pub const GPEDS1:    *mut u32 = (GPIO_BASE + 0x44) as _;
pub const GPHEN0:    *mut u32 = (GPIO_BASE + 0x64) as _;
pub const GPHEN1:    *mut u32 = (GPIO_BASE + 0x68) as _;
pub const GPPUD:     *mut u32 = (GPIO_BASE + 0x94) as _;
pub const GPPUDCLK0: *mut u32 = (GPIO_BASE + 0x98) as _;
pub const GPPUDCLK1: *mut u32 = (GPIO_BASE + 0x9C) as _;
