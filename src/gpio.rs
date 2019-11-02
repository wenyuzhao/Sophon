

#[cfg(feature="raspi3")]
pub const PERIPHERAL_BASE: usize = 0x3F000000;
#[cfg(feature="raspi4")]
pub const PERIPHERAL_BASE: usize = 0xFE000000;

// const GPIO_BASE: usize = PERIPHERAL_BASE + 0x200000;
