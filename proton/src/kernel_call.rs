use crate::*;
use super::page::{Page, Frame};
use super::address::Address;



#[repr(u64)]
pub enum KernelCall {
    Fork = 0,
    Exit,
    Sleep,
    MapPhysicalMemory,

    #[allow(non_camel_case_types)]
    __MAX_COUNT,
}

impl KernelCall {
    pub const COUNT: usize = Self::__MAX_COUNT as u64 as _;

    #[inline]
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

    #[inline]
    pub fn map_physical_memory(page: Page, frame: Frame) -> Result<Page, ()> {
        let message = Message::new(TaskId::NULL, TaskId::KERNEL, KernelCall::MapPhysicalMemory as _)
            .with_data((frame, page));
        message.send();
        let reply = Message::receive(Some(TaskId::KERNEL));
        let addr = reply.get_data::<Address>();
        if addr.is_zero() || *addr != page.start() {
            // use super::log::log;
            #[cfg(feature="user")]
            log!("Return {:?}, page = {:?}", addr, page);
            Err(())
        } else {
            Ok(page)
        }
    }

    #[inline]
    pub fn sleep() -> Result<(), ()> {
        let message = Message::new(TaskId::NULL, TaskId::KERNEL, KernelCall::Sleep as _);
        message.send();
        let _reply = Message::receive(Some(TaskId::KERNEL));
        Ok(())
    }
}
