
pub const ARM_GICD_BASE: usize = super::timer::ARM_TIMER_BASE;
pub const ARM_GICC_BASE: usize = super::timer::ARM_TIMER_BASE + 0x10000;


pub const IRQ_LINES: usize = 256;

macro_rules! u32_array {
    ($start: literal - $end: literal) => {
        [u32; ($end - $start + 4) / 4]
    };
}

macro_rules! pad {
    ($curr_end: literal - $next_start: literal) => {
        [u8; $next_start - $curr_end - 4]
    };
    (bytes: $bytes: literal) => {
        [u8; $bytes]
    };
}

#[repr(C)]
#[allow(non_snake_case)]
pub struct GICD {
    pub CTLR:       u32,    /* 0x0000 */          _0:  pad![0x0000 - 0x0080],
    pub IGROUPR:    u32_array![0x0080 - 0x00F8],  _1:  pad![bytes: 1],
    pub ISENABLER:  u32_array![0x0100 - 0x0178],  _2:  pad![bytes: 1],
    pub ICENABLER:  u32_array![0x0180 - 0x01F8],  _3:  pad![bytes: 1],
    pub ISPENDR:    u32_array![0x0200 - 0x0278],  _4:  pad![bytes: 1],
    pub ICPENDR:    u32_array![0x0280 - 0x02F8],  _5:  pad![bytes: 1],
    pub ISACTIVER:  u32_array![0x0300 - 0x0378],  _6:  pad![bytes: 1],
    pub ICACTIVER:  u32_array![0x0380 - 0x03F8],  _7:  pad![bytes: 1],
    pub IPRIORITYR: u32_array![0x0400 - 0x07DC],  _8:  pad![0x07DC - 0x0800],
    pub ITARGETSR:  u32_array![0x0800 - 0x0BDC],  _9:  pad![0x0BDC - 0x0C00],
    pub ICFGR:      u32_array![0x0C00 - 0x0CF4],  _10: pad![0x0CF4 - 0x0F00],
    pub SGIR:       u32,    /* 0x0F00 */
}

impl GICD {
	pub const CTLR_DISABLE: u32 = 0 << 0;
	pub const CTLR_ENABLE: u32 = 1 << 0;
	pub const CTLR_ENABLE_GROUP0: u32 = 1 << 0;
	pub const CTLR_ENABLE_GROUP1: u32 = 1 << 1;
	pub const IPRIORITYRAULT: u32 = 0xA0;
	pub const IPRIORITYR_FIQ: u32 = 0x40;
	pub const ITARGETSR_CORE0: u32 = 1 << 0;
	pub const ICFGR_LEVEL_SENSITIVE: u32 = 0 << 1;
	pub const ICFGR_EDGE_TRIGGERED: u32 = 1 << 1;
	pub const SGIR_SGIINTID__MASK: u32 = 0x0F;
	pub const SGIR_CPU_TARGET_LIST__SHIFT: u32 = 16;
    pub const SGIR_TARGET_LIST_FILTER__SHIFT: u32 = 24;

    pub fn get() -> &'static mut GICD {
        unsafe { ::core::mem::transmute(ARM_GICD_BASE) }
    }
}

#[repr(C)]
#[allow(non_snake_case)]
pub struct GICC {
    pub CTLR: u32, // 0x000
    pub PMR: u32, // 0x004;
    _0: pad![0x004 - 0x00C],
    pub IAR: u32, // 0x00C
    pub EOIR: u32, // 0x010
}

impl GICC {
	pub const CTLR_DISABLE: u32 = 0 << 0;
	pub const CTLR_ENABLE: u32 = 1 << 0;
	pub const CTLR_ENABLE_GROUP0: u32 = 1 << 0;
	pub const CTLR_ENABLE_GROUP1: u32 = 1 << 1;
	pub const CTLR_FIQ_ENABLE: u32 = 1 << 3;
	pub const PMR_PRIORITY: u32 = 0xF0 << 0;
    pub const IAR_INTERRUPT_ID__MASK: u32 = 0x3FF;
	pub const IAR_CPUID__SHIFT: u32 = 10;
	pub const IAR_CPUID__MASK: u32 = 3 << 10;
	pub const EOIR_EOIINTID__MASK: u32 = 0x3FF;
	pub const EOIR_CPUID__SHIFT: u32 = 10;
    pub const EOIR_CPUID__MASK: u32 = 3 << 10;

    pub fn get() -> &'static mut GICC {
        unsafe { ::core::mem::transmute(ARM_GICC_BASE) }
    }
}
