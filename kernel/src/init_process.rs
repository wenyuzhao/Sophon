use crate::task;

static INIT_ELF: &'static [u8] = include_bytes!("../../target/init");

pub extern fn entry() -> ! {
    println!("Init process start (kernel mode)");
    task::exec::exec_user(INIT_ELF);
}



