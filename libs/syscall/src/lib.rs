#![no_std]
#![feature(format_args_nl)]
#![feature(never_type)]

extern crate alloc;

#[macro_use]
mod log;
mod syscall;

use core::marker::PhantomData;

pub use crate::log::UserLogger;
pub use syscall::*;

pub trait Payload {
    fn decode(data: usize) -> Self;
    fn encode(&self) -> usize;
}

impl Payload for usize {
    fn decode(data: usize) -> Self {
        data
    }
    fn encode(&self) -> usize {
        *self
    }
}

impl Payload for u32 {
    fn decode(data: usize) -> Self {
        data as _
    }
    fn encode(&self) -> usize {
        *self as _
    }
}

impl<T: Sized> Payload for &T {
    fn decode(data: usize) -> Self {
        unsafe { &*(data as *const T) }
    }
    fn encode(&self) -> usize {
        *self as *const T as _
    }
}

impl<T: Sized> Payload for &mut T {
    fn decode(data: usize) -> Self {
        unsafe { &mut *(data as *mut T) }
    }
    fn encode(&self) -> usize {
        *self as *const T as _
    }
}

impl Payload for &str {
    fn decode(data: usize) -> Self {
        unsafe { *(data as *const &str) }
    }
    fn encode(&self) -> usize {
        self as *const &str as _
    }
}

impl<T> Payload for &[T] {
    fn decode(data: usize) -> Self {
        unsafe { *(data as *const &[T]) }
    }
    fn encode(&self) -> usize {
        self as *const &[T] as _
    }
}

impl<T> Payload for &mut [T] {
    fn decode(data: usize) -> Self {
        unsafe { *(data as *mut &mut [T]) }
    }
    fn encode(&self) -> usize {
        self as *const &mut [T] as _
    }
}

impl Payload for () {
    fn decode(_: usize) -> Self {
        ()
    }
    fn encode(&self) -> usize {
        0
    }
}

#[repr(C)]
pub struct RawModuleRequest<'a>(pub usize, pub [usize; 3], PhantomData<&'a usize>);

impl<'a> RawModuleRequest<'a> {
    #[inline]
    pub fn new(id: usize, a: &'a impl Payload, b: &'a impl Payload, c: &'a impl Payload) -> Self {
        let buf = [a.encode(), b.encode(), c.encode()];
        RawModuleRequest(id, buf, PhantomData)
    }
    #[inline]
    pub fn from_buf(x: [usize; 4]) -> Self {
        Self(x[0], [x[1], x[2], x[3]], PhantomData)
    }
    #[inline]
    pub fn as_buf(&self) -> [usize; 4] {
        [self.0, self.1[0], self.1[1], self.1[2]]
    }
    #[inline]
    pub fn id(&self) -> usize {
        self.0
    }
    #[inline]
    pub fn arg<V: Payload>(&self, i: usize) -> V {
        V::decode(self.1[i])
    }
}

pub trait ModuleRequest<'a> {
    fn as_raw(&'a self) -> RawModuleRequest<'a>;
    fn from_raw(raw: RawModuleRequest<'a>) -> Self;
}

impl<'a> ModuleRequest<'a> for ! {
    fn as_raw(&'a self) -> RawModuleRequest<'a> {
        unimplemented!()
    }
    fn from_raw(_: RawModuleRequest<'a>) -> Self {
        unimplemented!()
    }
}
