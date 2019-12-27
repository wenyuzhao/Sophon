use crate::ipc::IPC;

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
    pub kind: usize,
    data: [u64; 5],
}

impl Message {
    pub fn new(sender: TaskId, receiver: TaskId, kind: usize) -> Self {
        Self { sender, receiver, kind, data: [0; 5] }
    }
    pub fn with_data<T>(mut self, data: T) -> Self {
        self.set_data(data);
        self
    }
    pub fn set_data<T>(&mut self, data: T) {
        debug_assert!(::core::mem::size_of::<T>() <= ::core::mem::size_of::<[u64; 5]>());
        unsafe {
            let data_ptr: *mut T = &mut self.data as *mut [u64; 5] as usize as *mut T;
            data_ptr.write(data);
        }
    }
    pub fn get_data<T>(&self) -> &T {
        debug_assert!(::core::mem::size_of::<T>() <= ::core::mem::size_of::<[u64; 5]>());
        unsafe { ::core::mem::transmute(&self.data) }
    }


    pub fn send(self) {
        IPC::send(self);
    }

    pub fn receive(src: Option<TaskId>) -> Message {
        IPC::receive(src)
    }

    pub fn reply<T>(&self, data: T) {
        let n = Message::new(self.receiver, self.sender, self.kind).with_data(data);
        IPC::send(n);
    }
}