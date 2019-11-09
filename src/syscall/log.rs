use crate::exception::ExceptionFrame;
use crate::task::*;

pub fn log(exception_frame: &mut ExceptionFrame) -> isize {
    let string_pointer = exception_frame.x1 as *const &str;
    print!("{}", unsafe { *string_pointer });
    0
}
