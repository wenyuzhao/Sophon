use core::fmt;
use core::ops::*;
use core::hash::Hash;
use core::marker::PhantomData;
use core::convert::From;

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

impl <K: MemoryKind> Address<K> {
    #[inline]
    pub const fn new(v: usize) -> Self {
        Self(v, PhantomData)
    }

    #[inline]
    pub const fn null() -> Self {
        Self(0, PhantomData)
    }
    
    #[inline]
    pub fn is_null(&self) -> bool {
        self.0 == 0
    }

    #[inline]
    pub unsafe fn load<T: Copy>(&self) -> T {
        *(self.0 as *mut T)
    }

    #[inline]
    pub unsafe fn store<T: Copy>(&self, value: T) {
        *(self.0 as *mut T) = value;
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

    #[inline]
    pub fn validate(&self) -> Self {
        if cfg!(debug_assertions) {
            let b47 = **self & (0b1 << 47);
            const HIGH_MASK: usize = 0xFFFF << 48;
            if b47 == 0 {
                assert!((**self & HIGH_MASK) == 0, "Invalid address {:?}", self);
            } else {
                assert!((**self & HIGH_MASK) == HIGH_MASK, "Invalid address {:?}", self);
            }
        }
        *self
    }
}

impl <K: MemoryKind> Deref for Address<K> {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl <K: MemoryKind> From<usize> for Address<K> {
    fn from(v: usize) -> Self {
        Self(v, PhantomData).validate()
    }
}

impl <K: MemoryKind, T> From<*const T> for Address<K> {
    fn from(v: *const T) -> Self {
        Self(v as _, PhantomData).validate()
    }
}

impl <K: MemoryKind, T> From<*mut T> for Address<K> {
    fn from(v: *mut T) -> Self {
        Self(v as _, PhantomData).validate()
    }
}

impl <K: MemoryKind> fmt::Debug for Address<K> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:#x}", self.0)
    }
}

impl <K: MemoryKind> Add<usize> for Address<K> {
    type Output = Self;
    fn add(self, rhs: usize) -> Self {
        Self(self.0 + rhs, PhantomData)
    }
}

impl <K: MemoryKind> Sub<usize> for Address<K> {
    type Output = Self;
    fn sub(self, rhs: usize) -> Self {
        Self(self.0 - rhs, PhantomData)
    }
}

impl <K: MemoryKind> Add<isize> for Address<K> {
    type Output = Self;
    fn add(self, rhs: isize) -> Self {
        Self((self.0 as isize + rhs) as _, PhantomData)
    }
}

impl <K: MemoryKind> Sub<i32> for Address<K> {
    type Output = Self;
    fn sub(self, rhs: i32) -> Self {
        Self((self.0 as isize - (rhs as isize)) as _, PhantomData)
    }
}

impl <K: MemoryKind> Add<i32> for Address<K> {
    type Output = Self;
    fn add(self, rhs: i32) -> Self {
        self + rhs as isize
    }
}

impl <K: MemoryKind> Sub<Address<K>> for Address<K> {
    type Output = usize;
    fn sub(self, rhs: Address<K>) -> usize {
        self.0 - rhs.0
    }
}

impl <K: MemoryKind> AddAssign<usize> for Address<K> {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs;
    }
}

impl <K: MemoryKind> SubAssign<usize> for Address<K> {
    fn sub_assign(&mut self, rhs: usize) {
        self.0 -= rhs;
    }
}

impl <K: MemoryKind> AddAssign<isize> for Address<K> {
    fn add_assign(&mut self, rhs: isize) {
        self.0 = (self.0 as isize + rhs) as _;
    }
}

impl <K: MemoryKind> AddAssign<i32> for Address<K> {
    fn add_assign(&mut self, rhs: i32) {
        self.0 = (self.0 as isize + (rhs as isize)) as _;
    }
}

impl <K: MemoryKind> SubAssign<i32> for Address<K> {
    fn sub_assign(&mut self, rhs: i32) {
        self.0 = ((self.0 as isize) - (rhs as isize)) as _;
    }
}