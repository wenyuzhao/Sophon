mod page_table;

pub use page_table::*;

use crate::address::*;
use crate::page::*;
use bitflags::bitflags;
use core::fmt::Debug;

#[allow(unused, non_camel_case_types)]
#[bitflags(u64)]
pub enum PageFlags {
    PRESENT = 0b01,          // map a 4k page
    SMALL_PAGE = 0b10,       // map a 4k page
    USER = 1 << 6,           // enable EL0 Access
    NO_WRITE = 1 << 7,       // readonly
    ACCESSED = 1 << 10,      // accessed
    NO_EXEC = 1 << 54,       // no execute
    INNER_SHARE = 0b10 << 8, // outter shareable
    OUTER_SHARE = 0b11 << 8, // inner shareable
    COPY_ON_WRITE = 1 << 53,
    NORMAL_MEMORY = 0b001 << 2,
    DEVICE_MEMORY = 0b000 << 2,
    NO_CACHE = 0b10 << 2,
}

impl PageFlags {
    pub fn page_table_flags() -> PageFlags {
        PageFlags::NORMAL_MEMORY
            | PageFlags::PRESENT
            | PageFlags::SMALL_PAGE
            | PageFlags::OUTER_SHARE
            | PageFlags::ACCESSED
            | PageFlags::USER
    }
    pub fn kernel_data_flags<S: PageSize>() -> PageFlags {
        let mut flags = PageFlags::NORMAL_MEMORY
            | PageFlags::PRESENT
            | PageFlags::ACCESSED
            | PageFlags::OUTER_SHARE;
        if S::BYTES == Size4K::BYTES {
            flags = flags | PageFlags::SMALL_PAGE;
        }
        flags
    }
    pub fn kernel_data_flags_1g() -> PageFlags {
        Self::kernel_data_flags::<Size1G>()
    }
    pub fn kernel_data_flags_2m() -> PageFlags {
        Self::kernel_data_flags::<Size2M>()
    }
    pub fn kernel_data_flags_4k() -> PageFlags {
        Self::kernel_data_flags::<Size4K>()
    }
    pub fn kernel_code_flags_1g() -> PageFlags {
        Self::kernel_code_flags_2m()
    }
    pub fn kernel_code_flags_2m() -> PageFlags {
        PageFlags::NORMAL_MEMORY | PageFlags::PRESENT | PageFlags::ACCESSED | PageFlags::OUTER_SHARE
    }
    pub fn kernel_code_flags_4k() -> PageFlags {
        Self::kernel_code_flags_2m() | PageFlags::SMALL_PAGE
    }
    pub fn user_code_flags_2m() -> PageFlags {
        Self::kernel_code_flags_2m() | PageFlags::USER
    }
    pub fn user_code_flags_4k() -> PageFlags {
        Self::kernel_code_flags_4k() | PageFlags::USER
    }
    pub fn user_data_flags_4k() -> PageFlags {
        Self::kernel_data_flags_4k() | PageFlags::USER
    }
    pub fn user_stack_flags() -> PageFlags {
        PageFlags::NORMAL_MEMORY
            | PageFlags::PRESENT
            | PageFlags::SMALL_PAGE
            | PageFlags::OUTER_SHARE
            | PageFlags::ACCESSED
            | PageFlags::USER
    }
    pub fn device() -> PageFlags {
        PageFlags::DEVICE_MEMORY
            | PageFlags::PRESENT
            | PageFlags::SMALL_PAGE
            | PageFlags::OUTER_SHARE
            | PageFlags::ACCESSED
    }
}

#[cfg(not(target_pointer_width = "64"))]
compile_error!("Only supports 64bit machines");

#[repr(C)]
#[derive(Clone)]
pub struct PageTableEntry(pub(crate) u64);

impl core::fmt::Debug for PageTableEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.0 != 0 {
            write!(f, "{:#x?} {:?}", self.address(), self.flags())
        } else {
            write!(f, "{:#x?}", self.0)
        }
    }
}

impl PageTableEntry {
    const ADDRESS_MASK: u64 = 0x0000_ffff_ffff_f000;
    const FLAGS_MASK: u64 = !Self::ADDRESS_MASK;

    pub fn clear(&mut self) {
        unsafe {
            core::ptr::write_volatile(&mut self.0, 0);
        }
    }
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }
    pub fn present(&self) -> bool {
        self.flags().contains(PageFlags::PRESENT)
    }
    pub fn is_block(&self) -> bool {
        !self.flags().contains(PageFlags::SMALL_PAGE)
    }
    pub fn address(&self) -> Address<P> {
        ((unsafe { core::ptr::read_volatile(&self.0) } & Self::ADDRESS_MASK) as usize).into()
    }
    pub fn flags(&self) -> PageFlags {
        let v = unsafe { core::ptr::read_volatile(&self.0) } & Self::FLAGS_MASK;
        PageFlags::from(v)
    }
    pub fn update_flags(&mut self, new_flags: PageFlags) {
        let v = self.address().as_usize() as u64 | new_flags.value;
        unsafe {
            core::ptr::write_volatile(&mut self.0, v);
        }
    }
    pub fn set<S: PageSize>(&mut self, frame: Frame<S>, flags: PageFlags) {
        if S::BYTES != Size4K::BYTES {
            debug_assert!(flags.value & 0b10 == 0);
        } else {
            debug_assert!(flags.value & 0b10 == 0b10);
        }
        let mut a = frame.start().as_usize();
        a &= !(0xffff_0000_0000_0000);
        let v = a as u64 | flags.value;
        unsafe {
            core::ptr::write_volatile(&mut self.0, v);
        }
    }
}

pub trait TableLevel: Debug + 'static {
    const ID: usize;
    const SHIFT: usize;
    type NextLevel: TableLevel;
}

#[derive(Debug)]
pub struct L4;

impl TableLevel for L4 {
    const ID: usize = 4;
    const SHIFT: usize = 12 + 9 * 3;
    type NextLevel = L3;
}

#[derive(Debug)]
pub struct L3;

impl TableLevel for L3 {
    const ID: usize = 3;
    const SHIFT: usize = 12 + 9 * 2;
    type NextLevel = L2;
}

#[derive(Debug)]
pub struct L2;

impl TableLevel for L2 {
    const ID: usize = 2;
    const SHIFT: usize = 12 + 9 * 1;
    type NextLevel = L1;
}

#[derive(Debug)]
pub struct L1;

impl TableLevel for L1 {
    const ID: usize = 1;
    const SHIFT: usize = 12 + 9 * 0;
    type NextLevel = !;
}

impl TableLevel for ! {
    const ID: usize = 0;
    const SHIFT: usize = 0;
    type NextLevel = !;
}
