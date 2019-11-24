use crate::exception::ExceptionFrame;
use crate::task::*;

pub fn send(exception_frame: &mut ExceptionFrame) -> isize {
    let mut msg = unsafe { *(exception_frame.x1 as *const Message) };
    let current_task = Task::current().unwrap();
    msg.sender = current_task.id();
    Task::send_message(msg);
    0
}

pub fn receive(exception_frame: &mut ExceptionFrame) -> isize {
    let from_id = unsafe {
        let id = ::core::mem::transmute::<_, isize>(exception_frame.x1);
        if id < 0 {
            None
        } else {
            Some(::core::mem::transmute::<_, TaskId>(id))
        }
    };
    println!("{:?} start receive from {:?}", Task::current().unwrap().id(), from_id);
    Task::current().unwrap().context.exception_frame = exception_frame as _;
    let msg_slot = unsafe { exception_frame.x2 as *mut Message };
    Task::receive_message(from_id, unsafe { &mut *(exception_frame.x2 as *mut Message) });
    0
}
