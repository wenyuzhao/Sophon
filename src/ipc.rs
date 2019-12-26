use crate::arch::*;
use crate::task::*;

#[repr(usize)]
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
enum IPCCall {
    Log = 0x0,
    Send,
    Receive,
    #[allow(non_camel_case_types)] __MAX_SYSCALLS,
}

type Handler = fn (x0: usize, x1: usize, x2: usize, x3: usize, x4: usize, x5: usize) -> isize;

macro_rules! handlers {
    ($($f: expr,)*) => { handlers![$($f),*] };
    ($($f: expr),*) => {[
        $(|x0: usize, x1: usize, x2: usize, x3: usize, x4: usize, x5: usize| unsafe { ::core::mem::transmute($f(x0, x1, x2, x3, x4, x5)) }),*
    ]};
}

static IPC_CALL_HANDLERS: [Handler; IPCCall::__MAX_SYSCALLS as usize] = handlers![
    log,
    send,
    receive,
];

pub fn init() {
    Target::Interrupt::set_handler(InterruptId::Soft, Some(handle_syscall));
}

fn handle_syscall(x0: usize, x1: usize, x2: usize, x3: usize, x4: usize, x5: usize) -> isize {
    let syscall_id: IPCCall = unsafe { ::core::mem::transmute(x0) };
    let handler = IPC_CALL_HANDLERS[syscall_id as usize];
    handler(x0, x1, x2, x3, x4, x5)
}



// =====================
// ===   IPC Calls   ===
// =====================

pub fn log(_x0: usize, x1: usize, _x2: usize, _x3: usize, _x4: usize, _x5: usize) -> isize {
    let string_pointer = x1 as *const &str;
    print!("{}", unsafe { *string_pointer });
    0
}

fn send(_x0: usize, x1: usize, _x2: usize, _x3: usize, _x4: usize, _x5: usize) -> isize {
    let mut msg = unsafe { *(x1 as *const Message) };
    let current_task = Task::current().unwrap();
    msg.sender = current_task.id();
    Task::send_message(msg)
}

fn receive(_x0: usize, x1: usize, _x2: usize, _x3: usize, _x4: usize, _x5: usize) -> isize {
    let from_id = unsafe {
        let id = ::core::mem::transmute::<_, isize>(x1);
        if id < 0 {
            None
        } else {
            Some(::core::mem::transmute::<_, TaskId>(id))
        }
    };
    println!("{:?} start receiving from {:?}", Task::current().unwrap().id(), from_id);
    Task::receive_message(from_id)
}
