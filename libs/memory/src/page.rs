//! Abstractions for default-sized and huge virtual memory pages.

use super::address::*;
use core::cmp::Ordering;
use core::fmt;
use core::iter::Step;
use core::marker::PhantomData;
use core::ops::Range;

pub trait PageSize: 'static + Sized {
    const NAME: &'static str;
    const LOG_BYTES: usize;
    const BYTES: usize = 1 << Self::LOG_BYTES;
    const MASK: usize = Self::BYTES - 1;
}

pub struct Size4K;

impl PageSize for Size4K {
    const NAME: &'static str = "4K";
    const LOG_BYTES: usize = 12;
}

pub struct Size2M;

impl PageSize for Size2M {
    const NAME: &'static str = "2M";
    const LOG_BYTES: usize = 21;
}
pub struct Size1G;

impl PageSize for Size1G {
    const NAME: &'static str = "1G";
    const LOG_BYTES: usize = 30;
}

#[repr(transparent)]
pub struct Page<S: PageSize = Size4K, K: MemoryKind = V>(usize, PhantomData<(S, K)>);

pub type Frame<S = Size4K> = Page<S, P>;

impl<S: PageSize, K: MemoryKind> Page<S, K> {
    pub const LOG_BYTES: usize = S::LOG_BYTES;
    pub const BYTES: usize = 1 << Self::LOG_BYTES;
    pub const MASK: usize = Self::BYTES - 1;

    pub const fn new(a: Address<K>) -> Self {
        debug_assert!(Self::is_aligned(a));
        let page = Self(a.as_usize(), PhantomData);
        page
    }

    pub const fn containing(a: Address<K>) -> Self {
        Self::new(Self::align(a))
    }

    pub const fn align(a: Address<K>) -> Address<K> {
        Address::new(a.as_usize() & !Self::MASK)
    }

    pub const fn is_aligned(a: Address<K>) -> bool {
        (a.as_usize() & Self::MASK) == 0
    }

    pub const fn start(&self) -> Address<K> {
        Address::from(self.0)
    }

    pub const fn end(&self) -> Address<K> {
        self.start() + Self::BYTES
    }

    pub const fn range(&self) -> Range<Address<K>> {
        Range {
            start: self.start(),
            end: self.end(),
        }
    }

    #[inline]
    pub unsafe fn zero(&self) {
        core::ptr::write_bytes::<u8>(self.start().as_mut_ptr(), 0, Self::BYTES);
    }
}

impl<S: PageSize, K: MemoryKind> fmt::Debug for Page<S, K> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<0x{:x} {}>", self.0, S::NAME)
    }
}

unsafe impl<S: PageSize, K: MemoryKind> const Send for Page<S, K> {}
unsafe impl<S: PageSize, K: MemoryKind> const Sync for Page<S, K> {}

impl<S: PageSize, K: MemoryKind> const Clone for Page<S, K> {
    fn clone(&self) -> Self {
        Self(self.0, PhantomData)
    }

    fn clone_from(&mut self, source: &Self) {
        *self = source.clone()
    }
}

impl<S: PageSize, K: MemoryKind> const Copy for Page<S, K> {}

impl<S: PageSize, K: MemoryKind> const PartialEq for Page<S, K> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn ne(&self, other: &Self) -> bool {
        !self.eq(other)
    }
}

impl<S: PageSize, K: MemoryKind> const Eq for Page<S, K> {
    fn assert_receiver_is_total_eq(&self) {}
}

impl<S: PageSize, K: MemoryKind> const PartialOrd for Page<S, K> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }

    fn lt(&self, other: &Self) -> bool {
        matches!(self.partial_cmp(other), Some(Ordering::Less))
    }

    fn le(&self, other: &Self) -> bool {
        matches!(
            self.partial_cmp(other),
            Some(Ordering::Less | Ordering::Equal)
        )
    }

    fn gt(&self, other: &Self) -> bool {
        matches!(self.partial_cmp(other), Some(Ordering::Greater))
    }

    fn ge(&self, other: &Self) -> bool {
        matches!(
            self.partial_cmp(other),
            Some(Ordering::Greater | Ordering::Equal)
        )
    }
}

impl<S: PageSize, K: MemoryKind> const Ord for Page<S, K> {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.0, other.0) {
            (x, y) if x == y => Ordering::Equal,
            (x, y) if x < y => Ordering::Less,
            _ => Ordering::Greater,
        }
    }

    fn max(self, other: Self) -> Self {
        match Self::cmp(&self, &other) {
            Ordering::Less | Ordering::Equal => other,
            Ordering::Greater => self,
        }
    }

    fn min(self, other: Self) -> Self {
        match Self::cmp(&self, &other) {
            Ordering::Less | Ordering::Equal => self,
            Ordering::Greater => other,
        }
    }

    fn clamp(self, min: Self, max: Self) -> Self {
        assert!(min <= max);
        if self < min {
            min
        } else if self > max {
            max
        } else {
            self
        }
    }
}

impl<S: PageSize, K: MemoryKind> const Step for Page<S, K> {
    fn steps_between(start: &Self, end: &Self) -> Option<usize> {
        if start.0 > end.0 {
            None
        } else {
            Some((end.start() - start.start()) >> Self::LOG_BYTES)
        }
    }

    fn forward_checked(start: Self, count: usize) -> Option<Self> {
        Some(Self::new(start.start() + (count << Self::LOG_BYTES)))
    }

    fn backward_checked(start: Self, count: usize) -> Option<Self> {
        Some(Self::new(start.start() - (count << Self::LOG_BYTES)))
    }

    fn forward(start: Self, count: usize) -> Self {
        Step::forward_checked(start, count).unwrap()
    }

    unsafe fn forward_unchecked(start: Self, count: usize) -> Self {
        Step::forward(start, count)
    }
    fn backward(start: Self, count: usize) -> Self {
        Step::backward_checked(start, count).unwrap()
    }
    unsafe fn backward_unchecked(start: Self, count: usize) -> Self {
        Step::backward(start, count)
    }
}

/// Single page allocator.
pub trait PageAllocator<K: MemoryKind> {
    fn alloc<S: PageSize>(&self) -> Option<Page<S, K>>;
    fn dealloc<S: PageSize>(&self, page: Page<S, K>);
}

/// Page allocator for allocating and deallocating contiguous multiple pages.
/// Page start address is aligned to next-power-of-two.
pub trait PageResource<K: MemoryKind> {
    fn acquire_pages<S: PageSize>(&self, pages: usize) -> Option<Range<Page<S, K>>>;
    fn release_pages<S: PageSize>(&self, pages: Range<Page<S, K>>);
}

pub trait UnalignedPageResource<K: MemoryKind> {
    fn acquire_unaligned_pages<S: PageSize>(&self, pages: usize) -> Option<Range<Page<S, K>>>;
    fn release_unaligned_pages<S: PageSize>(&self, pages: Range<Page<S, K>>);
}
