#[cfg(feature = "kernel")]
pub mod ipc;
#[cfg(feature = "kernel")]
pub mod scheduler;
#[cfg(feature = "kernel")]
pub mod task;

#[cfg(feature = "kernel")]
pub use task::*;

#[derive(Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Clone, Copy)]
pub struct TaskId(pub usize);

impl TaskId {
    pub const NULL: Self = Self(0);
    pub const KERNEL: Self = Self(0);
}

#[repr(C, align(64))]
#[derive(Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Clone)]
pub struct Message {
    pub sender: TaskId,
    pub receiver: TaskId, // None for all tasks
    data: [u64; 6],
}

impl Message {
    #[inline]
    pub fn new(sender: TaskId, receiver: TaskId) -> Self {
        Self {
            sender,
            receiver,
            data: [0; 6],
        }
    }

    #[inline]
    pub fn with_data<T>(mut self, data: T) -> Self {
        self.set_data(data);
        self
    }

    #[inline]
    pub fn set_data<T>(&mut self, data: T) {
        debug_assert!(::core::mem::size_of::<T>() <= ::core::mem::size_of::<[u64; 5]>());
        unsafe {
            let data_ptr: *mut T = &mut self.data as *mut [u64; 6] as usize as *mut T;
            data_ptr.write(data);
        }
    }

    #[inline]
    pub fn get_data<T>(&self) -> &T {
        debug_assert!(::core::mem::size_of::<T>() <= ::core::mem::size_of::<[u64; 6]>());
        unsafe { ::core::mem::transmute(&self.data) }
    }

    #[inline]
    pub fn send(self) {
        unimplemented!()
        // crate::user::ipc::send(self);
    }

    #[inline]
    pub fn receive(src: Option<TaskId>) -> Message {
        unimplemented!()
        // crate::user::ipc::receive(src)
    }

    #[inline]
    pub fn reply<T>(&self, data: T) {
        unimplemented!()
        // let m = Message::new(self.receiver, self.sender).with_data(data);
        // crate::user::ipc::send(m);
    }
}
