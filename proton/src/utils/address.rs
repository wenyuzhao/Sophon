use core::convert::From;
use core::fmt;
use core::hash::Hash;
use core::marker::PhantomData;
use core::ops::*;

pub trait MemoryKind: Copy + Clone + PartialOrd + Ord + PartialEq + Eq + Hash {}

#[derive(Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Virtual;

pub type V = Virtual;

impl MemoryKind for Virtual {}

#[derive(Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Physical;

pub type P = Physical;

impl MemoryKind for Physical {}

#[repr(C)]
#[derive(Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Address<K: MemoryKind = Virtual>(usize, PhantomData<K>);

impl<K: MemoryKind> Address<K> {
    pub const ZERO: Self = Address::new(0usize);
    #[cfg(target_pointer_width = "32")]
    pub const LOG_SIZE: usize = 2;
    #[cfg(target_pointer_width = "64")]
    pub const LOG_SIZE: usize = 3;
    pub const SIZE: usize = 1 << Self::LOG_SIZE;

    #[inline]
    pub const fn new(v: usize) -> Self {
        Self(v, PhantomData)
    }

    #[inline]
    pub fn is_zero(&self) -> bool {
        self.0 == 0
    }

    #[inline]
    pub unsafe fn load<T: Copy>(&self) -> T {
        ::core::intrinsics::volatile_load(self.0 as *mut T)
    }

    #[inline]
    pub unsafe fn store<T: Copy>(&self, value: T) {
        ::core::intrinsics::volatile_store(self.0 as *mut T, value)
    }

    #[inline]
    pub const fn as_usize(&self) -> usize {
        self.0
    }

    #[inline]
    pub const fn from_usize(v: usize) -> Self {
        Self(v, PhantomData)
    }

    #[inline]
    pub fn as_ptr<T>(&self) -> *const T {
        unsafe { ::core::mem::transmute(self.0) }
    }

    #[inline]
    pub fn as_ptr_mut<T>(&self) -> *mut T {
        unsafe { ::core::mem::transmute(self.0) }
    }

    #[inline]
    pub unsafe fn as_ref<T>(&self) -> &'static T {
        ::core::mem::transmute(self.0)
    }

    #[inline]
    pub unsafe fn as_ref_mut<T>(&self) -> &'static mut T {
        ::core::mem::transmute(self.0)
    }
}

impl<K: MemoryKind> From<usize> for Address<K> {
    fn from(v: usize) -> Self {
        Self(v, PhantomData)
    }
}

impl<K: MemoryKind, T> From<*const T> for Address<K> {
    fn from(v: *const T) -> Self {
        Self(v as _, PhantomData)
    }
}

impl<K: MemoryKind, T> From<*mut T> for Address<K> {
    fn from(v: *mut T) -> Self {
        Self(v as _, PhantomData)
    }
}

impl Into<usize> for Address {
    #[inline(always)]
    fn into(self) -> usize {
        self.0
    }
}

impl<T> Into<*const T> for Address {
    #[inline(always)]
    fn into(self) -> *const T {
        self.0 as _
    }
}

impl<T> Into<*mut T> for Address {
    #[inline(always)]
    fn into(self) -> *mut T {
        self.0 as _
    }
}

impl<K: MemoryKind> fmt::Debug for Address<K> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}

macro_rules! impl_address_add {
    ($t: ty, $apply: expr) => {
        impl<K: MemoryKind> Add<$t> for Address<K> {
            type Output = Self;
            #[inline(always)]
            fn add(self, rhs: $t) -> Self {
                $apply(self, rhs)
            }
        }
        impl<K: MemoryKind> AddAssign<$t> for Address<K> {
            #[inline(always)]
            fn add_assign(&mut self, rhs: $t) {
                *self = *self + rhs;
            }
        }
    };
}

impl_address_add!(usize, |l: Address<_>, r| Address(l.0 + r, PhantomData));
impl_address_add!(isize, |l: Address<_>, r| Address(
    (l.0 as isize + r) as _,
    PhantomData
));
impl_address_add!(i32, |l, r| l + r as isize);

macro_rules! impl_address_sub {
    ($t: ty, $apply: expr) => {
        impl<K: MemoryKind> Sub<$t> for Address<K> {
            type Output = Self;
            #[inline(always)]
            fn sub(self, rhs: $t) -> Self {
                $apply(self, rhs)
            }
        }
        impl<K: MemoryKind> SubAssign<$t> for Address<K> {
            #[inline(always)]
            fn sub_assign(&mut self, rhs: $t) {
                *self = *self - rhs;
            }
        }
    };
}

impl_address_sub!(usize, |l: Address<_>, r| Address(l.0 - r, PhantomData));
impl_address_sub!(isize, |l: Address<_>, r| Address(
    (l.0 as isize - r) as _,
    PhantomData
));
impl_address_sub!(i32, |l, r| l - r as isize);
impl<K: MemoryKind> Sub<Address<K>> for Address<K> {
    type Output = usize;
    #[inline(always)]
    fn sub(self, rhs: Address<K>) -> usize {
        self.0 - rhs.0
    }
}
