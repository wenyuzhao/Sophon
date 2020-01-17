//! Abstractions for default-sized and huge virtual memory pages.

use core::fmt;
use core::marker::PhantomData;
use core::ops::*;
use super::address::*;
use core::iter::Step;
use core::hash::Hash;

pub trait PageSize: Copy + Clone + PartialOrd + Ord + PartialEq + Eq + Hash {
    const NAME: &'static str;
    const LOG_SIZE: usize;
    const SIZE: usize = 1 << Self::LOG_SIZE;
    const MASK: usize = Self::SIZE - 1;
}

#[derive(Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Size4K;

impl PageSize for Size4K {
    const NAME: &'static str = "4K";
    const LOG_SIZE: usize = 12;
}

#[derive(Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Size2M;

impl PageSize for Size2M {
    const NAME: &'static str = "2M";
    const LOG_SIZE: usize = 21;
}

#[repr(C)]
#[derive(Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Page<S: PageSize = Size4K, K: MemoryKind = V>(Address<K>, PhantomData<S>);

pub type Frame<S = Size4K> = Page<S, P>;

impl <S: PageSize, K: MemoryKind> Page<S, K> {
    pub const LOG_SIZE: usize = S::LOG_SIZE;
    pub const SIZE: usize = 1 << Self::LOG_SIZE;
    pub const MASK: usize = Self::SIZE - 1;
    pub const ZERO: Self = Self(Address::ZERO, PhantomData);

    pub fn range(start: Self, end: Self, mut f: impl FnMut(Self)) {
        let mut p = start;
        while p.start().as_usize() < end.start().as_usize() {
            f(p);
            p = Self(p.start() + Self::SIZE, PhantomData);
        }
    }

    pub fn range_inclusive(start: Self, end: Self, mut f: impl FnMut(Self)) {
        let mut p = start;
        while p.start().as_usize() <= end.start().as_usize() {
            f(p);
            p = Self(p.start() + Self::SIZE, PhantomData);
        }
    }

    #[inline]
    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    #[inline]
    pub fn start(&self) -> Address<K> {
        self.0
    }

    #[inline]
    pub fn end(&self) -> Address<K> {
        self.0 + Self::SIZE
    }

    #[inline]
    pub const fn align(a: Address<K>) -> Address<K> {
        Address::new(a.as_usize() & !Self::MASK)
    }

    #[inline]
    pub fn is_aligned(a: Address<K>) -> bool {
        (a.as_usize() & Self::MASK) == 0
    }

    #[inline]
    pub const fn of(a: Address<K>) -> Self {
        Self(Self::align(a), PhantomData)
    }

    #[inline]
    pub fn new(a: Address<K>) -> Self {
        let page = Self(a, PhantomData);
        debug_assert!(Self::is_aligned(page.0), "{:?} is not aligned", a);
        page
    }

    #[inline]
    pub unsafe fn zero(&self) {
        debug_assert!(!self.is_zero());
        let mut cursor = self.start();
        let limit = self.end();
        while cursor < limit {
            cursor.store(0usize);
            cursor = cursor + ::core::mem::size_of::<usize>();
        }
    }

    #[inline]
    pub fn align_up<M: MemoryKind>(a: Address<M>) -> Address<M> {
        let v = (a.as_usize() + S::SIZE - 1) & !(S::SIZE - 1);
        v.into()
    }
}

impl <S: PageSize, K: MemoryKind> fmt::Debug for Page<S, K> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<{:?} {}>", self.0, S::NAME)
    }
}

impl <S: PageSize, K: MemoryKind> Sub<Page<S, K>> for Page<S, K> {
    type Output = usize;
    fn sub(self, rhs: Self) -> usize {
        self.0 - rhs.0
    }
}


impl <S: PageSize, K: MemoryKind> Step for Page<S, K> {
    #[inline]
    fn steps_between(start: &Self, end: &Self) -> Option<usize> {
        if start > end {
            None
        } else {
            Some((*end - *start) >> Self::LOG_SIZE)
        }
    }
    fn replace_one(&mut self) -> Self {
        unimplemented!()
    }
    fn replace_zero(&mut self) -> Self {
        unimplemented!()
    }
    #[inline]
    fn add_one(&self) -> Self {
        Self(self.0 + Self::SIZE, PhantomData)
    }
    #[inline]
    fn sub_one(&self) -> Self {
        Self(self.0 - Self::SIZE, PhantomData)
    }
    #[inline]
    fn add_usize(&self, n: usize) -> Option<Self> {
        Some(Self(self.0 + (n << Self::LOG_SIZE), PhantomData))
    }
}