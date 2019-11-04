use crate::gpio::*;
use spin::Once;


const RNG_CTRL:     *mut u32 = (PERIPHERAL_BASE + 0x104000) as _;
const RNG_STATUS:   *mut u32 = (PERIPHERAL_BASE + 0x104004) as _;
const RNG_DATA:     *mut u32 = (PERIPHERAL_BASE + 0x104008) as _;
const RNG_INT_MASK: *mut u32 = (PERIPHERAL_BASE + 0x104010) as _;

static INITIALIZE: Once<()> = Once::INIT;

/// Generate a random integer within range [min, max)
pub fn random(min: usize, max: usize) -> usize {
    INITIALIZE.call_once(|| unsafe {
        *RNG_STATUS = 0x40000;
        *RNG_INT_MASK |= 1;
        *RNG_CTRL |= 1;
        // while ((*RNG_STATUS) >> 24) == 0 {
        //     asm!("nop"::::"volatile");
        // }
    });
    return (unsafe { *RNG_DATA } as usize % (max - min)) + min;
}