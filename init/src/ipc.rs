use crate::syscall::{SysCall, syscall};


#[derive(Debug, Hash, Eq, PartialEq, Ord, PartialOrd, Clone, Copy)]
pub struct Message {
    pub sender: usize,
    pub receiver: usize, // None for all tasks
    pub kind: usize,
    pub data: [u64; 16],
}

impl Message {
    pub fn send(self) {
        let message_ptr = &self as *const Self as usize;
        let ret = unsafe {
            syscall(SysCall::Send, [message_ptr, 0, 0, 0, 0, 0])
        };
        // assert!(ret == 0);
    }

    pub fn receive(src: Option<usize>) -> Message {
        let mut m = Message {
            sender: 0,
            receiver: 0,
            kind: 0,
            data: [0; 16]
        };
        let message_ptr = &mut m as *mut Self as usize;
        let src = src.map(|x| x as isize).unwrap_or(-1);
        // log!("Init start receiving fork");
        let ret = unsafe {
            syscall(SysCall::Receive, [::core::mem::transmute(src), message_ptr, 0, 0, 0, 0])
        };
        // assert!(ret == 0);
        m
    }

    pub fn get_data<T: Copy>(&self, word_index: usize) -> T {
        unsafe {
            let slot = &self.data[word_index] as &u64 as *const u64 as usize;
            *(slot as *const T)
        }
    }
}

// Commonly used kernel messages
pub mod kernel {
    use super::*;
    const KERNEL_TASK_ID: usize = 0;
    // const PID: usize = 0;
    const FORK: usize = 0;
    const EXIT: usize = 1;

    pub fn fork() -> isize {
        let msg = Message {
            sender: 0,
            receiver: KERNEL_TASK_ID,
            kind: FORK,
            data: [0; 16]
        };
        msg.send();
        let reply = Message::receive(Some(KERNEL_TASK_ID));
        // log!("Received from kernel {:?}", reply);
        reply.get_data(0)
    }
}