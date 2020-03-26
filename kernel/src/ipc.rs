use crate::task::*;
use proton::IPC;
use crate::*;
use crate::arch::*;
use core::marker::PhantomData;

pub struct IPCController<K: AbstractKernel> {
    phantom: PhantomData<K>,
}

impl <K: AbstractKernel> IPCController<K> {
    pub const fn new() -> Self {
        Self { phantom: PhantomData }
    }

    fn handle(&self, ipc: IPC, args: [usize; 5]) -> isize {
        let [a, b, c, d, e] = args;
        match ipc {
            IPC::Log => log::<K>(a, b, c, d, e),
            IPC::Send => send::<K>(a, b, c, d, e),
            IPC::Receive => receive::<K>(a, b, c, d, e),
        }
    }
}

// type Handler = fn (x0: usize, x1: usize, x2: usize, x3: usize, x4: usize, x5: usize) -> isize;

// macro_rules! handlers {
//     ($($f: expr,)*) => { handlers![$($f),*] };
//     ($($f: expr),*) => {[
//         $(|x0: usize, x1: usize, x2: usize, x3: usize, x4: usize, x5: usize| unsafe { ::core::mem::transmute($f(x0, x1, x2, x3, x4, x5)) }),*
//     ]};
// }

// static IPC_CALL_HANDLERS: [Handler; IPC::COUNT] = handlers![
//     log,
//     send,
//     receive,
// ];

pub fn init<K: AbstractKernel>() {
    <K::Arch as AbstractArch>::Interrupt::set_handler(InterruptId::Soft, Some(box |a, b, c, d, e, f| {
        let ipc: IPC = unsafe { ::core::mem::transmute(a) };
        K::global().ipc.handle(ipc, [b, c, d, e, f])
    }));
}

// fn handle_syscall<K: AbstractKernel>(x0: usize, x1: usize, x2: usize, x3: usize, x4: usize, x5: usize) -> isize {
//     let syscall_id: IPC = unsafe { ::core::mem::transmute(x0) };
//     let handler = IPC_CALL_HANDLERS[syscall_id as usize];
//     handler(x0, x1, x2, x3, x4, x5)
// }



// =====================
// ===   IPC Calls   ===
// =====================

pub fn log<K: AbstractKernel>(x1: usize, _x2: usize, _x3: usize, _x4: usize, _x5: usize) -> isize {
    let string_pointer = x1 as *const &str;
    let s: &str = unsafe { &*string_pointer };
    crate::debug::_print::<K>(format_args!("{}", s));
    0
}

fn send<K: AbstractKernel>(x1: usize, _x2: usize, _x3: usize, _x4: usize, _x5: usize) -> isize {
    let mut msg = unsafe { (*(x1 as *const Message)).clone() };
    let current_task = Task::<K>::current().unwrap();
    msg.sender = current_task.id();
    Task::<K>::send_message(msg)
}

fn receive<K: AbstractKernel>(x1: usize, _x2: usize, _x3: usize, _x4: usize, _x5: usize) -> isize {
    let from_id = unsafe {
        let id = ::core::mem::transmute::<_, isize>(x1);
        if id < 0 {
            None
        } else {
            Some(::core::mem::transmute::<_, TaskId>(id))
        }
    };
    debug!(K: "{:?} start receiving from {:?}", Task::<K>::current().unwrap().id(), from_id);
    Task::<K>::receive_message(from_id)
}
