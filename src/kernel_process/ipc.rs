use crate::task::*;

pub fn send(mut m: Message) {
    m.sender = Task::current().unwrap().id();
    unsafe {
        asm!("svc #0"::"{x0}"(1), "{x1}"(&mut m as *mut Message): "x0" "x1" "memory");
    }
}

pub fn receive(_from: Option<TaskId>) -> Message {
    unsafe {
        let mut msg: Message = ::core::mem::zeroed();
        asm!("svc #0"::"{x0}"(2), "{x1}"(-1isize), "{x2}"(&mut msg as *mut Message):"x0" "x1" "x2" "memory");
        msg
    }
}