use super::address::*;
use super::page::*;

pub const KERNEL_CORE0_STACK_START: usize = 0xffff0000_0007c000;
pub const KERNEL_CORE0_STACK_END:   usize = 0xffff0000_00080000;

/// Kernel process stack
pub const KERNEL_STACK_START: Address<V> = Address::new(0x1000);
pub const KERNEL_STACK_PAGES: usize = 8; // Too many???
pub const KERNEL_STACK_SIZE: usize = KERNEL_STACK_PAGES * Size4K::SIZE;
pub const KERNEL_STACK_END: Address<V> = Address::new(KERNEL_STACK_START.as_usize() + KERNEL_STACK_SIZE);

/// User heap layout
pub const USER_STACK_START: Address<V> = Address::new(0x111900000);
pub const USER_STACK_PAGES: usize = 4; // Too many???
pub const USER_STACK_SIZE: usize = USER_STACK_PAGES * Size4K::SIZE;
pub const USER_STACK_END: Address<V> = Address::new(USER_STACK_START.as_usize() + USER_STACK_SIZE);
pub const USER_CODE_START: Page = Page::of(USER_STACK_END);

pub const KERNEL_HEAP_SIZE: usize = 16 * 1024 * 1024; // 16M

pub const KERNEL_START: usize = 0x80000; // 16M

#[inline]
pub fn kernel_end() -> usize {
    unsafe { &__kernel_end as *const _ as usize }
}

#[inline]
pub fn kernel_heap_start() -> usize {
    Frame::<Size2M>::align_up::<P>(kernel_end().into()).as_usize()
}

#[inline]
pub fn kernel_heap_end() -> usize {
    kernel_heap_start() + KERNEL_HEAP_SIZE
}

pub const MMIO_START: usize = crate::gpio::PERIPHERAL_BASE - 0x1000000;
pub const MMIO_END: usize = MMIO_START + 0x1000000;

pub const LOG_MAX_HEAP_SIZE: usize = 30; // 1G
pub const MAX_HEAP_SIZE: usize = 1 << LOG_MAX_HEAP_SIZE; // 1G

extern {
    static __kernel_start: usize;
    static __kernel_end: usize;
}