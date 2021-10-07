#![no_std]

use core::ops::Range;

use memory::{address::Address, page::Frame};

pub struct BootInfo {
    pub available_physical_memory: &'static [Range<Frame>],
    pub device_tree: &'static [u8],
    pub init_fs: &'static [u8],
    pub uart: Option<Address>,
}
