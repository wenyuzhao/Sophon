use core::ops::Deref;

use alloc::vec::Vec;

pub struct ProcessorLocalStorage<T: Default> {
    data: Vec<T>,
}

impl<T: Default> ProcessorLocalStorage<T> {
    /// Create a new per-processor storage.
    /// This must be called after the arch-dependent initialization is finished.
    pub fn new() -> Self {
        let len = Self::num_cores();
        Self {
            data: (0..len).map(|_| T::default()).collect(),
        }
    }

    /// Get stroage by processor index.
    #[inline(always)]
    pub fn get(&self, index: usize) -> &T {
        &self.data[index]
    }

    pub fn num_cores() -> usize {
        1
    }

    pub fn current_core() -> usize {
        0
    }
}

impl<T: Default> Deref for ProcessorLocalStorage<T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.data[Self::current_core()]
    }
}
