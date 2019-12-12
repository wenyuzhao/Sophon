use crate::task::*;

pub fn send(mut m: Message) {
    m.sender = Task::current().unwrap().id();
    Task::send_message(m)
}

pub fn receive(from: Option<TaskId>) -> Message {
    let mut m = unsafe { ::core::mem::zeroed() };
    Task::receive_message(from, &mut m);
    m
}