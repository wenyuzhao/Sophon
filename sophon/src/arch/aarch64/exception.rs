use crate::arch::{aarch64::context::*, *};
use crate::memory::kernel::KERNEL_MEMORY_MAPPER;
use crate::memory::physical::PHYSICAL_MEMORY;
use crate::modules::INTERRUPT;
use crate::task::sched::SCHEDULER;
use crate::task::PROCESS_MANAGER;
use core::arch::{asm, global_asm};
use cortex_a::{asm::barrier, registers::*};
use memory::page::{Page, Size1G, Size2M, Size4K};
use memory::page_table::PageFlags;
use tock_registers::interfaces::{Readable, Writeable};

#[repr(usize)]
#[derive(Debug)]
#[allow(unused)]
pub enum ExceptionLevel {
    EL0 = 0,
    EL1 = 1,
    EL2 = 2,
}

#[repr(usize)]
#[derive(Debug)]
#[allow(unused)]
pub enum ExceptionKind {
    Synchronous = 0,
    IRQ = 1,
    FIQ = 2,
    SError = 3,
}

#[repr(u32)]
#[derive(Debug)]
#[allow(unused)]
pub enum ExceptionClass {
    SVCAArch64 = 0b010101,
    DataAbortLowerEL = 0b100100,
    DataAbortHigherEL = 0b100101,
}

#[repr(C)]
#[derive(Debug)]
pub struct ExceptionFrame {
    pub q: [u128; 32],
    pub elr_el1: *mut u8,
    pub spsr_el1: usize,
    pub x30: usize,
    /// This holds sp_el0
    pub sp_el0: usize,
    pub x28: usize,
    pub x29: usize,
    pub x26: usize,
    pub x27: usize,
    pub x24: usize,
    pub x25: usize,
    pub x22: usize,
    pub x23: usize,
    pub x20: usize,
    pub x21: usize,
    pub x18: usize,
    pub x19: usize,
    pub x16: usize,
    pub x17: usize,
    pub x14: usize,
    pub x15: usize,
    pub x12: usize,
    pub x13: usize,
    pub x10: usize,
    pub x11: usize,
    pub x8: usize,
    pub x9: usize,
    pub x6: usize,
    pub x7: usize,
    pub x4: usize,
    pub x5: usize,
    pub x2: usize,
    pub x3: usize,
    pub x0: usize,
    pub x1: usize,
}

impl Default for ExceptionFrame {
    fn default() -> Self {
        Self {
            q: [0; 32],
            elr_el1: core::ptr::null_mut(),
            spsr_el1: 0,
            x30: 0,
            sp_el0: 0,
            x28: 0,
            x29: 0,
            x26: 0,
            x27: 0,
            x24: 0,
            x25: 0,
            x22: 0,
            x23: 0,
            x20: 0,
            x21: 0,
            x18: 0,
            x19: 0,
            x16: 0,
            x17: 0,
            x14: 0,
            x15: 0,
            x12: 0,
            x13: 0,
            x10: 0,
            x11: 0,
            x8: 0,
            x9: 0,
            x6: 0,
            x7: 0,
            x4: 0,
            x5: 0,
            x2: 0,
            x3: 0,
            x0: 0,
            x1: 0,
        }
    }
}

unsafe fn get_exception_class() -> ExceptionClass {
    let esr_el1: u32;
    asm!("mrs {:x}, esr_el1", out(reg) esr_el1);
    ::core::mem::transmute(esr_el1 >> 26)
}

unsafe fn is_el0(frame: &ExceptionFrame) -> bool {
    frame.spsr_el1 & 0b1111usize == 0
}

#[no_mangle]
pub unsafe extern "C" fn handle_exception(exception_frame: &mut ExceptionFrame) {
    let privileged = !is_el0(exception_frame);
    AArch64Context::of(&*SCHEDULER.get_current_task().unwrap())
        .push_exception_frame(exception_frame);
    let exception = get_exception_class();
    // trace!("Exception received {:?}", exception);
    match exception {
        ExceptionClass::SVCAArch64 => {
            // trace!(
            //     "SVCAArch64 Start {:?} #{} privileged={}",
            //     SCHEDULER.get_current_task().unwrap().id,
            //     exception_frame.x0,
            //     privileged
            // );
            let f = if privileged {
                crate::task::syscall::handle_syscall::<true>
            } else {
                crate::task::syscall::handle_syscall::<false>
            };
            let r = f(
                exception_frame.x0,
                exception_frame.x1,
                exception_frame.x2,
                exception_frame.x3,
                exception_frame.x4,
                exception_frame.x5,
            );
            exception_frame.x0 = ::core::mem::transmute(r);
            // trace!("SVCAArch64 End");
        }
        ExceptionClass::DataAbortLowerEL | ExceptionClass::DataAbortHigherEL => {
            let mut far: usize;
            asm!("mrs {:x}, far_el1", out(reg) far);
            let mut elr: usize;
            asm!("mrs {:x}, elr_el1", out(reg) elr);
            // trace!("TASK {:?}", SCHEDULER.get_current_task().unwrap().id);
            let proc = PROCESS_MANAGER.current_proc().unwrap();
            let task = SCHEDULER.get_current_task().unwrap();
            let proc_p4 = proc.mem.get_page_table() as *const PageTable;
            let curr_p4 = PageTable::get() as *const PageTable;
            let kern_p4 = KERNEL_MEMORY_MAPPER.get_kernel_page_table() as *const PageTable;
            let mut handled = false;
            {
                let pt = PageTable::get();
                let _guard = KERNEL_MEMORY_MAPPER.with_kernel_address_space();
                let fault_addr = Address::<V>::from(far);
                // Copy-on-write
                if let Some((_a, flags, level)) = pt.translate_with_flags(fault_addr) {
                    // trace!("FAULT_FLAGS {:?} {:?} {:?}", fault_addr, flags, a);
                    if flags.contains(PageFlags::COPY_ON_WRITE) {
                        // trace!("COW {:?} {:?}", fault_addr, flags);
                        match level {
                            1 => pt.copy_on_write::<Size4K>(
                                Page::containing(fault_addr),
                                &PHYSICAL_MEMORY,
                            ),
                            2 => pt.copy_on_write::<Size2M>(
                                Page::containing(fault_addr),
                                &PHYSICAL_MEMORY,
                            ),
                            3 => pt.copy_on_write::<Size1G>(
                                Page::containing(fault_addr),
                                &PHYSICAL_MEMORY,
                            ),
                            _ => unreachable!(),
                        }
                        handled = true;
                    }
                }
                // trace!(
                //     "PTE4#510 {:?} {:?}",
                //     pt.entries[510].address(),
                //     pt.entries[510].flags()
                // );
            }
            if !handled {
                error!(
                    "Data Abort: FAR={:?} ELR={:?} PRIV={:?} TID={:?} PID={:?}",
                    far as *mut (), elr as *mut (), privileged, task.id, proc.id
                );
                let translation = {
                    let _guard = KERNEL_MEMORY_MAPPER.with_kernel_address_space();
                    PageTable::get().translate_with_flags(far.into())
                };
                error!(
                    "PageTable: proc={:?} curr={:?} kernel={:?} translation={:?}",
                    proc_p4, curr_p4, kern_p4, translation
                );
                error!("SP_EL0: {:#x?}", exception_frame.sp_el0);
                error!("REGS: {:#x?}", exception_frame);
                panic!()
            }
        }
        #[allow(unreachable_patterns)]
        _ => panic_for_unhandled_exception(exception_frame),
    }
    // Note: `Task::current()` must be dropped before calling `return_to_user`.
    let context =
        AArch64Context::of(&*SCHEDULER.get_current_task().unwrap()) as *const AArch64Context;
    (*context).return_to_user();
}

#[no_mangle]
pub unsafe extern "C" fn handle_exception_serror(exception_frame: *mut ExceptionFrame) {
    info!("SError received");
    panic_for_unhandled_exception(exception_frame);
}

unsafe fn panic_for_unhandled_exception(exception_frame: *mut ExceptionFrame) -> ! {
    let exception = get_exception_class();
    info!(
        "Exception Frame: {:?} {:?}",
        exception_frame, *exception_frame
    );
    let far = FAR_EL1.get() as *mut ();
    let elr = ELR_EL1.get() as *mut ();
    let esr = ESR_EL1.get() as *mut ();
    let eebr0_el1 = TTBR0_EL1.get() as *mut ();
    let sp_el0 = SP_EL0.get() as *mut ();
    info!(
        "Abort FAR={:?} ELR={:?} TTBR0_EL0={:?} esr_el1={:?} SP_EL0={:?}",
        far, elr, eebr0_el1, esr as *mut (), sp_el0,
    );
    panic!(
        "Unknown exception 0b{:b}",
        ::core::mem::transmute::<_, u32>(exception)
    );
}

#[no_mangle]
pub unsafe extern "C" fn handle_interrupt(exception_frame: &mut ExceptionFrame) {
    INTERRUPT.interrupt_begin();
    let irq = INTERRUPT.get_active_irq().unwrap();
    AArch64Context::of(&*SCHEDULER.get_current_task().unwrap())
        .push_exception_frame(exception_frame);
    super::super::handle_irq(irq);
    INTERRUPT.interrupt_end();
    ::core::sync::atomic::fence(::core::sync::atomic::Ordering::SeqCst);
    // Note: `Task::current()` must be dropped before calling `return_to_user`.
    let context =
        AArch64Context::of(&*SCHEDULER.get_current_task().unwrap()) as *const AArch64Context;
    (*context).return_to_user();
}

extern "C" {
    pub fn exception_handlers() -> !;
    pub fn exit_exception() -> !;
}

pub unsafe extern "C" fn setup_vbar() {
    // log!("efi_main: {:?}", efi_main as *const fn());
    // log!("handle_exception: {:?}", exception::handle_exception as *const fn());
    // log!("exception_handlers: {:?}", exception::exception_handlers as *const fn());
    let v_ptr = exception_handlers as *const fn() as u64;
    // log!("exception_handlers: {:#x}", v_ptr);
    VBAR_EL1.set(v_ptr as u64);
    barrier::isb(barrier::SY);
}

// FIXME: We may need to switch stack after enter an exception,
//        to avoid stack overflow.
// Exception handlers table
global_asm! {"
.global exception_handlers
.global exit_exception

.macro push_all
    stp x0,  x1,  [sp, #-16]!
    stp x2,  x3,  [sp, #-16]!
    stp x4,  x5,  [sp, #-16]!
    stp x6,  x7,  [sp, #-16]!
    stp x8,  x9,  [sp, #-16]!
    stp x10, x11, [sp, #-16]!
    stp x12, x13, [sp, #-16]!
    stp x14, x15, [sp, #-16]!
    stp x16, x17, [sp, #-16]!
    stp x18, x19, [sp, #-16]!
    stp x20, x21, [sp, #-16]!
    stp x22, x23, [sp, #-16]!
    stp x24, x25, [sp, #-16]!
    stp x26, x27, [sp, #-16]!
    stp x28, x29, [sp, #-16]!
    mrs	x21, sp_el0
    mrs x22, elr_el1
    mrs x23, spsr_el1
    stp x30, x21, [sp, #-16]!
    stp x22, x23, [sp, #-16]!
    stp q0,  q1,  [sp, #-32]!
    stp q2,  q3,  [sp, #-32]!
    stp q4,  q5,  [sp, #-32]!
    stp q6,  q7,  [sp, #-32]!
    stp q8,  q9,  [sp, #-32]!
    stp q10, q11, [sp, #-32]!
    stp q12, q13, [sp, #-32]!
    stp q14, q15, [sp, #-32]!
    stp q16, q17, [sp, #-32]!
    stp q18, q19, [sp, #-32]!
    stp q20, q21, [sp, #-32]!
    stp q22, q23, [sp, #-32]!
    stp q24, q25, [sp, #-32]!
    stp q26, q27, [sp, #-32]!
    stp q28, q29, [sp, #-32]!
    stp q30, q31, [sp, #-32]!
.endm

.macro pop_all
    ldp q30, q31, [sp], #32
    ldp q28, q29, [sp], #32
    ldp q26, q27, [sp], #32
    ldp q24, q25, [sp], #32
    ldp q22, q23, [sp], #32
    ldp q20, q21, [sp], #32
    ldp q18, q19, [sp], #32
    ldp q16, q17, [sp], #32
    ldp q14, q15, [sp], #32
    ldp q12, q13, [sp], #32
    ldp q10, q11, [sp], #32
    ldp q8,  q9,  [sp], #32
    ldp q6,  q7,  [sp], #32
    ldp q4,  q5,  [sp], #32
    ldp q2,  q3,  [sp], #32
    ldp q0,  q1,  [sp], #32
    ldp x22, x23, [sp], #16
    ldp x30, x21, [sp], #16
    msr	sp_el0, x21
    msr elr_el1, x22
    msr spsr_el1, x23
    ldp x28, x29, [sp], #16
    ldp x26, x27, [sp], #16
    ldp x24, x25, [sp], #16
    ldp x22, x23, [sp], #16
    ldp x20, x21, [sp], #16
    ldp x18, x19, [sp], #16
    ldp x16, x17, [sp], #16
    ldp x14, x15, [sp], #16
    ldp x12, x13, [sp], #16
    ldp x10, x11, [sp], #16
    ldp x8,  x9,  [sp], #16
    ldp x6,  x7,  [sp], #16
    ldp x4,  x5,  [sp], #16
    ldp x2,  x3,  [sp], #16
    ldp x0,  x1,  [sp], #16
.endm

.macro except_hang, exception_id
    .align 7
0:  wfi
    b 0b
.endm

exit_exception:
    pop_all
    eret

except:
    push_all
    mov x0, sp
    bl handle_exception
    except_hang 0

serror:
    push_all
    mov x0, sp
    bl handle_exception_serror
    except_hang 0

irq:
    push_all
    mov x0, sp
    bl handle_interrupt
    except_hang 0

    .balign 4096
exception_handlers:
    // Same exeception level, EL0
    .align 9; b except
    .align 7; b irq
    .align 7; b serror
    .align 7; b serror
    // Same exeception level, ELx
    .align 9; b except
    .align 7; b irq
    .align 7; b serror
    .align 7; b serror
    // Transit to upper exeception level, AArch64
    .align 9; b except
    .align 7; b irq
    .align 7; b serror
    .align 7; b serror
    // Transit to upper exeception level, AArch32: Unreachable
    .align 9; b except
    .align 7; b irq
    .align 7; b serror
    .align 7; b serror
"}
