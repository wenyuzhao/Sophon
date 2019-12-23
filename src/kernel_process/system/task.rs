use super::super::ipc;
use crate::task::*;
use crate::arch::*;

pub fn fork(m: &Message) {
    println!("fork {:?}", m.sender);
    let parent_task = crate::task::Task::by_id(m.sender).unwrap();
    loop {
        debug_assert!(Target::Interrupt::is_enabled());
        let block_to_receive_from = parent_task.block_to_receive_from.lock();
        if block_to_receive_from.is_some() && block_to_receive_from.as_ref().unwrap().0 == Some(Task::current().unwrap().id()) {
            break
        }
        // println!("{:?}", block_to_receive_from);
    }
    println!("Fork task start");
    let child_task = Target::Interrupt::uninterruptable(|| parent_task.fork());
    println!("Fork task end");

    let mut reply_parent = *m;
    reply_parent.receiver = parent_task.id();
    reply_parent.data[0] = unsafe { ::core::mem::transmute(child_task.id()) };
    println!("Start send to {:?}", parent_task.id());
    ipc::send(reply_parent);
    println!("Finish send to {:?}", parent_task.id());
    
    let mut reply_child = *m;
    reply_child.receiver = child_task.id();
    reply_child.data[0] = 0;
    ipc::send(reply_child);
}


pub fn exit(m: &Message) {
    unimplemented!()
}
