
use proton::utils::volatile::Volatile;

#[cfg(feature="device-raspi3-qemu")]
pub const PERIPHERAL_BASE: usize = 0xFFFF0000_3F000000;
#[cfg(feature="device-raspi4")]
pub const PERIPHERAL_BASE: usize = 0xFFFF0000_FE000000;



pub trait MemoryMappedRegisters: Sized {
    const BASE: usize;
    const BASE_LOW: usize = Self::BASE & 0xffff_ffff_ffff;

    #[inline]
    fn get() -> &'static mut Self {
        unsafe { &mut *(Self::BASE as *mut Self) }
    }
    
    #[inline]
    fn get_low() -> &'static mut Self {
        unsafe { &mut *(Self::BASE_LOW as *mut Self) }
    }
}



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

impl MemoryMappedRegisters for GPIORegisters {
    const BASE: usize = PERIPHERAL_BASE + 0x200000;
}



#[repr(C)]
pub struct UARTRegisters {
    pub dr: Volatile<u32>,       // 0x00
    pub rsrecr: Volatile<u32>,   // 0x04
    _0: [u8; 16],                // 0x08
    pub fr: Volatile<u32>,       // 0x18,
    _1: [u8; 4],                 // 0x1c,
    pub ilpr: Volatile<u32>,     // 0x20,
    pub ibrd: Volatile<u32>,     // 0x24,
    pub fbrd: Volatile<u32>,     // 0x28,
    pub lcrh: Volatile<u32>,     // 0x2c,
    pub cr: Volatile<u32>,       // 0x30,
    pub ifls: Volatile<u32>,     // 0x34,
    pub imsc: Volatile<u32>,     // 0x38,
    pub ris: Volatile<u32>,      // 0x3c,
    pub mis: Volatile<u32>,      // 0x40,
    pub icr: Volatile<u32>,      // 0x44,
    pub dmacr: Volatile<u32>,    // 0x48,
}

impl MemoryMappedRegisters for UARTRegisters {
    const BASE: usize = PERIPHERAL_BASE + 0x201000;
}



#[repr(C)]
pub struct SystemTimerRegisters {
    pub cs: Volatile<u32>,    // 0x00
    pub clo: Volatile<u32>,   // 0x04
    pub chi: Volatile<u32>,   // 0x08
    pub c0: Volatile<u32>,   // 0x0c
    pub c1: Volatile<u32>,   // 0x10
    pub c2: Volatile<u32>,   // 0x14
    pub c3: Volatile<u32>,   // 0x18
}

impl MemoryMappedRegisters for SystemTimerRegisters {
    const BASE: usize = PERIPHERAL_BASE + 0x3000;
}



#[cfg(feature="device-raspi3-qemu")]
pub const ARM_TIMER_BASE: usize = 0xffff0000_40000000;
#[cfg(feature="device-raspi4")]
pub const ARM_TIMER_BASE: usize = 0xFFFF0000_FF800000;