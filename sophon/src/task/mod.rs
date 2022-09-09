pub mod proc;
pub mod runnable;
pub mod syscall;

pub use self::proc::*;
use crate::modules::PROCESS_MANAGER;
use ::proc::Runnable;
pub use ::proc::{ProcId, TaskId};

pub extern "C" fn entry(_ctx: *mut ()) -> ! {
    let runnable = unsafe {
        &mut *(PROCESS_MANAGER.current_task().unwrap().runnable() as *const dyn Runnable
            as *mut dyn Runnable)
    };
    runnable.run()
}
