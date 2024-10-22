use crate::Payload;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OpaqueMutexPointer(pub *mut ());

impl OpaqueMutexPointer {
    pub const fn cast<T>(&self) -> &T {
        unsafe { &*(self.0 as *const T) }
    }
    pub const fn cast_mut_ptr<T>(&self) -> *mut T {
        self.0 as _
    }
}

impl Payload for OpaqueMutexPointer {
    fn decode(data: usize) -> Self {
        Self(data as _)
    }
    fn encode(&self) -> usize {
        self.0 as _
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OpaqueCondvarPointer(pub *mut ());

impl OpaqueCondvarPointer {
    pub const fn cast<T>(&self) -> &T {
        unsafe { &*(self.0 as *const T) }
    }
    pub const fn cast_mut_ptr<T>(&self) -> *mut T {
        self.0 as _
    }
}

impl Payload for OpaqueCondvarPointer {
    fn decode(data: usize) -> Self {
        Self(data as _)
    }
    fn encode(&self) -> usize {
        self.0 as _
    }
}
