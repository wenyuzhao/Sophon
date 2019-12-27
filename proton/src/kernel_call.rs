use crate::*;



#[repr(u64)]
pub enum KernelCall {
    Fork = 0,
    Exit,
    PhysicalMemory,

    #[allow(non_camel_case_types)]
    __MAX_COUNT,
}

impl KernelCall {
    pub const COUNT: usize = Self::__MAX_COUNT as u64 as _;

    pub fn fork() -> Result<Option<TaskId>, ()> {
        let message = Message::new(TaskId::NULL, TaskId::KERNEL, KernelCall::Fork as _);
        message.send();
        let reply = Message::receive(Some(TaskId::KERNEL));
        let task_id = reply.get_data::<isize>();
        unsafe {
            if *task_id == 0 {
                Ok(None)
            } else if *task_id > 0 {
                Ok(Some(::core::mem::transmute(*task_id)))
            } else {
                Err(())
            }
        }
    }
}
