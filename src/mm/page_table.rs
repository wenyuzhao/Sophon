use cortex_a::regs::*;


bitflags! {
    pub struct PageFlags: usize {
        const PAGETABLE        = 0b11;      // map a 4k page
        const BLOCK       = 0b01;      // map a 2m block
        const PAGE        = 0b11;      // map a 4k page
        const USER        = 1 << 6;    // enable EL0 Access
        const NO_WRITE    = 1 << 7;    // readonly
        const ACCESSED    = 1 << 10;   // accessed
        const NO_EXEC     = 1 << 54;   // no execute
        const INNER_SHARE = 0b10 << 8; // outter shareable
        const OUTER_SHARE = 0b11 << 8; // inner shareable
    }
}

#[repr(C)]
struct PageTableEntry(usize);

impl PageTableEntry {
    pub fn address(&self) -> usize {
        self.0 & (0b111111111 << 39)
    }
    pub fn flags(&self) -> PageFlags {
        let v = self.0 & !(0b111111111 << 39);
        PageFlags::from_bits_truncate(v)
    }
    pub fn set(&mut self, address: usize, flags: PageFlags) {
        self.0 = address | flags.bits();
    }
}

#[repr(C, align(4096))]
struct PageTable {
    pub entries: [PageTableEntry; 512]
}