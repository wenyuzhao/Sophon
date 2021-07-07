use core::intrinsics::{volatile_load, volatile_store};

#[repr(transparent)]
pub struct Volatile<T: Copy>(T);

impl<T: Copy> Volatile<T> {
    pub const fn new(t: T) -> Self {
        Self(t)
    }

    #[inline]
    pub fn get(&self) -> T {
        unsafe { volatile_load(&self.0) }
    }

    #[inline]
    pub fn set(&mut self, value: T) {
        unsafe { volatile_store(&mut self.0, value) }
    }

    #[inline]
    pub fn update(&mut self, f: impl Fn(T) -> T) {
        let t = self.get();
        self.set(f(t));
    }
}
