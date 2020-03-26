use core::sync::atomic::{AtomicUsize, Ordering};
use core::ops::{Deref, DerefMut};
use core::mem::MaybeUninit;
use core::cell::Cell;

const UNINITIALIZED: usize = 0;
const INITIALIZING: usize = 1;
const INITIALIZED: usize = 2;

pub struct Lazy<T, F: FnOnce() -> T = fn() -> T> {
    state: AtomicUsize,
    init: Cell<Option<F>>,
    value: MaybeUninit<T>,
}

impl <T, F: FnOnce() -> T> Lazy<T, F> {
    pub const fn new(f: F) -> Self {
        Self {
            state: AtomicUsize::new(UNINITIALIZED),
            value: MaybeUninit::uninit(),
            init: Cell::new(Some(f)),
        }
    }

    fn force_initialize(&self) {
        let f: F = self.init.replace(None).unwrap();
        let v: T = f();
        unsafe { (self.value.as_ptr() as usize as *mut T).write(v) };
        self.state.store(INITIALIZED, Ordering::Relaxed);
    }

    #[inline(never)]
    fn force_slow(lazy: &Self) {
        let mut state = lazy.state.load(Ordering::Relaxed);
        loop {
            if state == INITIALIZED {
                return
            } else if state == UNINITIALIZED {
                let old_state = lazy.state.compare_and_swap(UNINITIALIZED, INITIALIZING, Ordering::Relaxed);
                if old_state == UNINITIALIZED {
                    lazy.force_initialize();
                    return
                }
            }
            state = lazy.state.load(Ordering::Relaxed);
        }
    }

    #[inline(always)]
    pub fn force(lazy: &Self) {
        if INITIALIZED == lazy.state.load(Ordering::Relaxed) {
            return
        }
        Self::force_slow(lazy);
    }
}

impl <T, F: FnOnce() -> T> Deref for Lazy<T, F> {
    type Target = T;
    fn deref(&self) -> &T {
        Lazy::force(self);
        unsafe { &*self.value.as_ptr() }
    }
}

impl <T, F: FnOnce() -> T> DerefMut for Lazy<T, F> {
    fn deref_mut(&mut self) -> &mut T {
        Lazy::force(self);
        unsafe { &mut *self.value.as_mut_ptr() }
    }
}

impl <T: Default> Default for Lazy<T> {
    fn default() -> Self {
        Lazy::new(T::default)
    }
}

unsafe impl <T, F: FnOnce() -> T> Send for Lazy<T, F> {}
unsafe impl <T, F: FnOnce() -> T> Sync for Lazy<T, F> {}
