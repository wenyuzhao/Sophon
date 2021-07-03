pub use crate::task::Message;
use crate::task::TaskId;

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
            asm!("svc #0", in("x0") Self::Log as usize, in("x1") &message as *const &str);
        }
    }

    #[inline]
    pub fn send(mut m: Message) {
        let ret: isize;
        unsafe {
            asm!("svc #0", inout("x0") Self::Send as usize => ret, in("x1") &mut m as *mut Message);
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
            asm!("svc #0", inout("x0") Self::Receive as usize => ret, in("x1") from_task, in("x2") &mut msg as *mut Message);
            assert!(ret == 0, "{:?}", ret);
            msg
        }
    }
}
