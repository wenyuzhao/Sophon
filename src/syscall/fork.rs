use crate::exception::ExceptionFrame;

pub fn fork(exception_frame: *mut ExceptionFrame) -> isize {
    let parent_task = crate::task::Task::current().unwrap();
    let child_task = parent_task.fork(exception_frame as *const ExceptionFrame as usize);
    return unsafe { ::core::mem::transmute(child_task.id()) }
}
