use crate::task::*;

pub fn send(x0: usize, x1: usize, x2: usize, x3: usize, x4: usize, x5: usize) -> isize {
    let mut msg = unsafe { *(x1 as *const Message) };
    let current_task = Task::current().unwrap();
    msg.sender = current_task.id();
    Task::send_message(msg);
    0
}

pub fn receive(x0: usize, x1: usize, x2: usize, x3: usize, x4: usize, x5: usize) -> isize {
    let from_id = unsafe {
        let id = ::core::mem::transmute::<_, isize>(x1);
        if id < 0 {
            None
        } else {
            Some(::core::mem::transmute::<_, TaskId>(id))
        }
    };
    println!("{:?} start receiving from {:?}", Task::current().unwrap().id(), from_id);
    let msg_slot = unsafe { x2 as *mut Message };
    Task::receive_message(from_id, unsafe { &mut *msg_slot });
    0
}
