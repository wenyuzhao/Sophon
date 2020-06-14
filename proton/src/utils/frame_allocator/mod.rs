use crate::address::*;
use crate::page::*;
use spin::Mutex;

pub trait FrameAllocator: Sized {
    fn identity_alloc<S: PageSize>(&mut self, frame: Frame<S>);
    fn alloc<S: PageSize>(&mut self) -> Frame<S>;
    fn free<S: PageSize>(&mut self, frame: Frame<S>);
}

pub struct SynchronizedFrameAllocator<FA: FrameAllocator> {
    pub fa: Mutex<FA>,
}

impl <FA: FrameAllocator> SynchronizedFrameAllocator<FA> {
    pub const fn new(fa: FA) -> Self {
        Self {
            fa: Mutex::new(fa),
        }
    }

    pub fn identity_alloc<S: PageSize>(&self, frame: Frame<S>) {
        self.fa.lock().identity_alloc(frame);
    }

    pub fn alloc<S: PageSize>(&self) -> Frame<S> {
        self.fa.lock().alloc()
    }

    pub fn free<S: PageSize>(&self, frame: Frame<S>) {
        self.fa.lock().free(frame)
    }
}

pub mod bump_allocator;