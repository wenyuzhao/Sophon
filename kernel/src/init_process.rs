use crate::task;

// static INIT_ELF: &'static [u8] = include_bytes!("../../target/init");

// pub extern fn entry() -> ! {
//     println!("Init process start (kernel mode)");
//     task::exec::exec_user(INIT_ELF);
// }

// static EMMC_ELF: &'static [u8] = include_bytes!("../../target/aarch64-proton/debug/emmc");

pub extern fn entry() -> ! {
    println!("Init process start (kernel mode)");
    task::exec::exec_user(EMMC_ELF);
}

