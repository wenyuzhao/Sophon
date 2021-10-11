use core::ops::Range;

use memory::address::Address;

pub mod kernel;
pub mod physical;

pub const USER_SPACE_MEMORY_RANGE: Range<Address> =
    Address::new(0x1000_00000000)..Address::new(0xf000_00000000);
