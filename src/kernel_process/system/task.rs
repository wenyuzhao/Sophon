use super::super::ipc;
use crate::task::*;
use crate::arch::*;

pub fn fork(m: &Message) {
    println!("fork {:?}", m.sender);
    let parent_task = crate::task::Task::by_id(m.sender).unwrap();
    loop {
        debug_assert!(Target::Interrupt::is_enabled());
        let block_to_receive_from = parent_task.block_to_receive_from.lock();
        if block_to_receive_from.is_some() && *block_to_receive_from.as_ref().unwrap() == Some(Task::current().unwrap().id()) {
            break
        }
    }
    println!("Fork task start");
    let child_task = Target::Interrupt::uninterruptable(|| parent_task.fork());
    println!("Fork task end");

    let reply_parent = Message::new(m.receiver, parent_task.id(), 0)
        .with_data(child_task.id());
    println!("Start send to {:?}", parent_task.id());
    ipc::send(reply_parent);
    println!("Finish send to {:?}", parent_task.id());
    
    let reply_child = Message::new(m.receiver, child_task.id(), 0);
    ipc::send(reply_child);
}


pub fn exit(_m: &Message) {
    unimplemented!()
}
