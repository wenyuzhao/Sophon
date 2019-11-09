use crate::exception::ExceptionFrame;
use crate::task::*;

pub fn fork(exception_frame: &mut ExceptionFrame) -> TaskId {
    let parent_task = crate::task::Task::current().unwrap();
    debug!("Fork task start");
    let child_task = parent_task.fork(exception_frame as *const ExceptionFrame as usize);
    debug!("Fork task");
    child_task.id()
}
