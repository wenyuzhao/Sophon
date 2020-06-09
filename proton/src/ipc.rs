use crate::*;

pub use super::Message;

#[repr(usize)]
pub enum IPC {
    Log = 0,
    Send,
    Receive,

    // #[allow(non_camel_case_types)]
    // __MAX_COUNT,
}

impl IPC {
    // pub const COUNT: usize = Self::__MAX_COUNT as _;
    
    #[inline]
    pub fn log(message: &str) {
        unsafe {
            llvm_asm!("svc #0"::"{x0}"(Self::Log as usize), "{x1}"(&message as *const &str): "x0" "x1" "memory");
        }
    }

    #[inline]
    pub fn send(mut m: Message) {
        let ret: isize;
        unsafe {
            llvm_asm!("svc #0":"={x0}"(ret):"{x0}"(Self::Send as usize), "{x1}"(&mut m as *mut Message): "x0" "x1" "memory");
        }
        assert!(ret == 0, "{:?}", ret);
    }

    #[inline]
    pub fn receive(from: Option<TaskId>) -> Message {
        unsafe {
            let mut msg: Message = ::core::mem::zeroed();
            let from_task: isize = match from {
                Some(t) => ::core::mem::transmute(t),
                None => -1,
            };
            let ret: isize;
            llvm_asm!("svc #0":"={x0}"(ret):"{x0}"(Self::Receive as usize), "{x1}"(from_task), "{x2}"(&mut msg as *mut Message):"x0" "x1" "x2" "memory");
            assert!(ret == 0, "{:?}", ret);
            msg
        }
    }
}
