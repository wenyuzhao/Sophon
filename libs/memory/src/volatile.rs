use core::cell::UnsafeCell;
use core::mem;
use core::ops::{Deref, DerefMut};
use core::ptr::{read_volatile, write_volatile};

#[repr(transparent)]
pub struct Volatile<T: Copy>(UnsafeCell<T>);

impl<T: Copy> Volatile<T> {
    #[inline(always)]
    pub fn get(&self) -> T {
        unsafe { read_volatile(self.0.get()) }
    }

    #[inline(always)]
    pub unsafe fn force_set(&self, v: T) {
        write_volatile(self.0.get(), v)
    }

    #[inline(always)]
    pub fn set(&mut self, v: T) {
        unsafe { write_volatile(self.0.get(), v) }
    }

    #[inline(always)]
    pub fn update(&mut self, f: impl Fn(T) -> T) {
        let t = self.get();
        self.set(f(t));
    }
}

#[repr(transparent)]
pub struct VolatileArray<T: Copy, const N: usize>([Volatile<T>; N]);

impl<T: Copy, const N: usize> VolatileArray<T, N> {
    #[inline(always)]
    pub fn get(&self, i: usize) -> T {
        self[i].get()
    }

    #[inline(always)]
    pub fn set(&mut self, i: usize, v: T) {
        self[i].set(v)
    }

    #[inline(always)]
    pub fn update(&mut self, i: usize, f: impl Fn(T) -> T) {
        self[i].update(f)
    }
}

pub type VolatileArrayForRange<T, const START: usize, const END: usize> =
    VolatileArray<T, { (END - START) / mem::size_of::<T>() }>;

pub type PaddingForRange<const START: usize, const END: usize> = [u8; END - START];

pub type PaddingForBytes<const N: usize> = [u8; N];

impl<T: Copy, const N: usize> Deref for VolatileArray<T, N> {
    type Target = [Volatile<T>; N];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Copy, const N: usize> DerefMut for VolatileArray<T, N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
