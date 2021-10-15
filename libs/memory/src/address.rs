use core::cmp::Ordering;
use core::convert::From;
use core::fmt;
use core::intrinsics::transmute;
use core::iter::Step;
use core::marker::PhantomData;
use core::ops::*;

pub trait MemoryKind: 'static + Sized {}

pub struct Virtual;

pub type V = Virtual;

impl MemoryKind for Virtual {}

pub struct Physical;

pub type P = Physical;

impl MemoryKind for Physical {}

#[repr(transparent)]
pub struct Address<K: MemoryKind = Virtual>(usize, PhantomData<K>);

impl<K: MemoryKind> Address<K> {
    pub const ZERO: Self = Address::new(0usize);
    #[cfg(target_pointer_width = "32")]
    pub const LOG_BYTES: usize = 2;
    #[cfg(target_pointer_width = "64")]
    pub const LOG_BYTES: usize = 3;
    pub const BYTES: usize = 1 << Self::LOG_BYTES;

    pub const fn new(v: usize) -> Self {
        Self(v, PhantomData)
    }

    pub const fn is_zero(&self) -> bool {
        self.0 == 0
    }

    pub const fn align_up(&self, align: usize) -> Self {
        debug_assert!(align.is_power_of_two());
        let mask = align - 1;
        Self::new((self.0 + mask) & !mask)
    }

    pub const fn align_down(&self, align: usize) -> Self {
        debug_assert!(align.is_power_of_two());
        let mask = align - 1;
        Self::new(self.0 & !mask)
    }

    pub const fn is_aligned_to(&self, align: usize) -> bool {
        debug_assert!(align.is_power_of_two());
        (self.0 & (align - 1)) == 0
    }

    pub const fn as_usize(&self) -> usize {
        self.0
    }

    pub const fn as_ptr<T>(&self) -> *const T {
        self.0 as _
    }

    pub const fn as_mut_ptr<T>(&self) -> *mut T {
        self.0 as _
    }

    pub const unsafe fn as_ref<T: 'static>(&self) -> &'static T {
        debug_assert!(!self.is_zero());
        &*self.as_ptr()
    }

    pub const unsafe fn as_mut<T: 'static>(&self) -> &'static mut T {
        debug_assert!(!self.is_zero());
        &mut *self.as_mut_ptr()
    }

    #[inline(always)]
    pub unsafe fn load<T: Copy>(&self) -> T {
        ::core::intrinsics::volatile_load(self.0 as *mut T)
    }

    #[inline(always)]
    pub unsafe fn store<T: Copy>(&self, value: T) {
        ::core::intrinsics::volatile_store(self.0 as *mut T, value)
    }

    #[inline]
    pub unsafe fn zero(range: Range<Self>) {
        let size = range.end - range.start;
        core::ptr::write_bytes::<u8>(range.start.as_mut_ptr(), 0, size);
    }
}

unsafe impl<K: MemoryKind> const Send for Address<K> {}
unsafe impl<K: MemoryKind> const Sync for Address<K> {}

impl<K: MemoryKind> const Clone for Address<K> {
    fn clone(&self) -> Self {
        Self(self.0, PhantomData)
    }

    fn clone_from(&mut self, source: &Self) {
        *self = source.clone()
    }
}

impl<K: MemoryKind> const Copy for Address<K> {}

impl<K: MemoryKind> const From<usize> for Address<K> {
    fn from(value: usize) -> Self {
        Self::new(value)
    }
}

impl<K: MemoryKind, T> const From<*const T> for Address<K> {
    fn from(value: *const T) -> Self {
        unsafe { Self::new(transmute(value)) }
    }
}

impl<K: MemoryKind, T> const From<*mut T> for Address<K> {
    fn from(value: *mut T) -> Self {
        unsafe { Self::new(transmute(value)) }
    }
}

impl<K: MemoryKind, T> const From<&T> for Address<K> {
    fn from(value: &T) -> Self {
        unsafe { Self::new(transmute(value as *const T)) }
    }
}

impl<K: MemoryKind, T> const From<&mut T> for Address<K> {
    fn from(value: &mut T) -> Self {
        unsafe { Self::new(transmute(value as *const T)) }
    }
}

impl<K: MemoryKind> const From<Address<K>> for usize {
    fn from(value: Address<K>) -> usize {
        value.0
    }
}

impl<K: MemoryKind, T> const From<Address<K>> for *const T {
    fn from(value: Address<K>) -> *const T {
        value.0 as _
    }
}

impl<K: MemoryKind, T> const From<Address<K>> for *mut T {
    fn from(value: Address<K>) -> *mut T {
        value.0 as _
    }
}

impl<K: MemoryKind> const Deref for Address<K> {
    type Target = usize;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<K: MemoryKind> const PartialEq for Address<K> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }

    fn ne(&self, other: &Self) -> bool {
        !self.eq(other)
    }
}

impl<K: MemoryKind> const Eq for Address<K> {
    fn assert_receiver_is_total_eq(&self) {}
}

impl<K: MemoryKind> const PartialOrd for Address<K> {
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

impl<K: MemoryKind> const Ord for Address<K> {
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

impl<K: MemoryKind> const Add<usize> for Address<K> {
    type Output = Self;
    fn add(self, other: usize) -> Self::Output {
        Self::new(*self + other)
    }
}

impl<K: MemoryKind> const AddAssign<usize> for Address<K> {
    fn add_assign(&mut self, other: usize) {
        *self = *self + other
    }
}

impl<K: MemoryKind> const Add<Self> for Address<K> {
    type Output = Self;
    fn add(self, other: Self) -> Self::Output {
        self + *other
    }
}

impl<K: MemoryKind> const AddAssign<Self> for Address<K> {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other
    }
}

impl<K: MemoryKind> const Add<isize> for Address<K> {
    type Output = Self;
    fn add(self, other: isize) -> Self::Output {
        Self::new((*self as isize + other) as usize)
    }
}

impl<K: MemoryKind> const AddAssign<isize> for Address<K> {
    fn add_assign(&mut self, other: isize) {
        *self = *self + other
    }
}

impl<K: MemoryKind> const Add<i32> for Address<K> {
    type Output = Self;
    fn add(self, other: i32) -> Self::Output {
        self + other as isize
    }
}

impl<K: MemoryKind> const AddAssign<i32> for Address<K> {
    fn add_assign(&mut self, other: i32) {
        *self = *self + other
    }
}

impl<K: MemoryKind> const Sub<Self> for Address<K> {
    type Output = usize;
    fn sub(self, other: Self) -> Self::Output {
        debug_assert!(self.0 >= other.0);
        *self - *other
    }
}

impl<K: MemoryKind> const Sub<usize> for Address<K> {
    type Output = Self;
    fn sub(self, other: usize) -> Self::Output {
        Self::new(self.0 - other)
    }
}

impl<K: MemoryKind> const SubAssign<usize> for Address<K> {
    fn sub_assign(&mut self, other: usize) {
        *self = *self - other
    }
}

impl<K: MemoryKind> fmt::Debug for Address<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.as_ptr::<u8>())
    }
}

impl<K: MemoryKind> const Step for Address<K> {
    fn steps_between(start: &Self, end: &Self) -> Option<usize> {
        if start.0 > end.0 {
            None
        } else {
            Some(*end - *start)
        }
    }

    fn forward_checked(start: Self, count: usize) -> Option<Self> {
        Some(start + count)
    }

    fn backward_checked(start: Self, count: usize) -> Option<Self> {
        Some(start - count)
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
