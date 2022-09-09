use crate::{ModuleRequest, Payload, RawModuleRequest};

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

pub enum ProcRequest {
    MutexCreate,
    MutexLock(OpaqueMutexPointer),
    MutexUnlock(OpaqueMutexPointer),
    MutexDestroy(OpaqueMutexPointer),
    CondvarCreate,
    CondvarWait(OpaqueCondvarPointer, OpaqueMutexPointer),
    CondvarNotifyAll(OpaqueCondvarPointer),
    CondvarDestroy(OpaqueCondvarPointer),
}

impl<'a> ModuleRequest<'a> for ProcRequest {
    fn as_raw(&'a self) -> RawModuleRequest<'a> {
        match self {
            Self::MutexCreate => RawModuleRequest::new(1, &(), &(), &()),
            Self::MutexLock(x) => RawModuleRequest::new(2, x, &(), &()),
            Self::MutexUnlock(x) => RawModuleRequest::new(3, x, &(), &()),
            Self::MutexDestroy(x) => RawModuleRequest::new(4, x, &(), &()),
            Self::CondvarCreate => RawModuleRequest::new(5, &(), &(), &()),
            Self::CondvarWait(x, y) => RawModuleRequest::new(6, x, y, &()),
            Self::CondvarNotifyAll(x) => RawModuleRequest::new(7, x, &(), &()),
            Self::CondvarDestroy(x) => RawModuleRequest::new(8, x, &(), &()),
        }
    }
    fn from_raw(raw: RawModuleRequest<'a>) -> Self {
        match raw.id() {
            1 => Self::MutexCreate,
            2 => Self::MutexLock(raw.arg(0)),
            3 => Self::MutexUnlock(raw.arg(0)),
            4 => Self::MutexDestroy(raw.arg(0)),
            5 => Self::CondvarCreate,
            6 => Self::CondvarWait(raw.arg(0), raw.arg(1)),
            7 => Self::CondvarNotifyAll(raw.arg(0)),
            8 => Self::CondvarDestroy(raw.arg(0)),
            _ => panic!("Unknown request"),
        }
    }
}
