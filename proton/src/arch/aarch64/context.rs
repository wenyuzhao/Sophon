use super::exception::ExceptionFrame;
use crate::memory::page_table::kernel::*;
use crate::task::Message;
use crate::utils::page::*;
use crate::{arch::*, heap::constants::*, memory::physical::*};
use core::ops::Range;
use cortex_a::regs::*;
// use

#[repr(C, align(4096))]
pub struct KernelStack {
    /// This page is protected to trap stack overflow
    guard: [u8; Size4K::BYTES],
    stack: [u8; KERNEL_STACK_SIZE],
}

impl KernelStack {
    pub fn new() -> &'static mut Self {
        let pages = KERNEL_STACK_PAGES + 1;
        let stack = PHYSICAL_PAGE_RESOURCE
            .lock()
            .acquire::<Size4K>(pages)
            .unwrap();
        let kernel_stack = unsafe { stack.start.start().as_mut::<Self>() };
        kernel_stack.init();
        kernel_stack
    }
    pub fn init(&mut self) {
        // let guard_page = Page::<Size4K, V>::new(Address::from(&self.guard as *const [u8; Size4K::SIZE]));
        // PageTable::<L4>::get(true).update_flags(guard_page, PageFlags::_KERNEL_STACK_GUARD_FLAGS);
        // let stack_page_start = Page::<Size4K, V>::new(Address::from(&self.stack as *const [u8; KERNEL_STACK_SIZE]));
        // let stack_page_end = stack_page_start.add_usize(KERNEL_STACK_PAGES).unwrap();
        // for stack_page in stack_page_start..stack_page_end {
        //     PageTable::<L4>::get(true).update_flags(stack_page, PageFlags::_KERNEL_STACK_FLAGS);
        // }
        for i in 0..KERNEL_STACK_SIZE {
            unsafe {
                ::core::intrinsics::volatile_store(&mut self.stack[i], 0);
            }
        }
    }
    pub const fn range(&self) -> Range<Address> {
        let start = Address::from(&self.stack as *const [u8; KERNEL_STACK_SIZE]);
        let end = start + KERNEL_STACK_SIZE;
        start..end
    }
}

impl Drop for KernelStack {
    // Unprotect stack pages
    fn drop(&mut self) {
        unreachable!()
        // let guard_page = Page::<Size4K, V>::new(Address::from(&self.guard as *const [u8; Size4K::SIZE]));
        // PageTable::<L4>::get(true).update_flags(guard_page, PageFlags::_KERNEL_DATA_FLAGS_4K);
        // let stack_page_start = Page::<Size4K, V>::new(Address::from(&self.stack as *const [u8; KERNEL_STACK_SIZE]));
        // let stack_page_end = ::core::iter::Step::forward(stack_page_start, KERNEL_STACK_PAGES);
        // for stack_page in stack_page_start..stack_page_end {
        //     PageTable::<L4>::get(true).update_flags(stack_page, PageFlags::_KERNEL_DATA_FLAGS_4K);
        // }
    }
}

/// Represents the archtectural context (i.e. registers)
#[allow(improper_ctypes)]
#[repr(C)]
pub struct AArch64Context {
    pub exception_frame: *mut ExceptionFrame,
    // sp: *mut u8,
    // x19_to_x29: [usize; 11],
    // x19: usize,
    // x20: usize,
    // x21: usize,
    // x22: usize,
    // x23: usize,
    // x24: usize,
    // x25: usize,
    // x26: usize,
    // x27: usize,
    // x28: usize,
    // x29: usize, // FP
    entry_pc: *mut u8, // x30

    // q: [u128; 32], // Neon registers
    pub p4: Address<P>,
    kernel_stack: Option<*mut KernelStack>,
    kernel_stack_top: *mut u8,
    response_message: Option<Message>,
    response_status: Option<isize>,
}

impl ArchContext for AArch64Context {
    fn empty() -> Self {
        unsafe { ::core::mem::zeroed() }
    }

    /// Create a new context with empty regs, given kernel stack,
    /// and current p4 table
    fn new(entry: *const extern "C" fn(a: *mut ()) -> !, ctx_ptr: *mut ()) -> Self {
        // Alloc kernel stack (SP_EL1)
        let kernel_stack = KernelStack::new();
        let sp: *mut u8 = kernel_stack.range().end.as_mut_ptr();
        let mut ctx = Self::empty();
        ctx.entry_pc = entry as _;
        ctx.kernel_stack_top = sp;
        ctx.p4 = KernelPageTable::get().into();
        ctx.kernel_stack = Some(kernel_stack);
        ctx.set_response_status(unsafe { ::core::mem::transmute(ctx_ptr) });
        ctx
    }

    fn set_page_table(&mut self, page_table: &'static mut KernelPageTable) {
        self.p4 = page_table.into();
    }

    fn set_response_message(&mut self, m: Message) {
        self.response_message = Some(m);
    }

    fn set_response_status(&mut self, s: isize) {
        self.response_status = Some(s);
    }

    unsafe extern "C" fn return_to_user(&mut self) -> ! {
        assert!(!TargetArch::interrupt().is_enabled());
        // Switch page table
        if self.p4.as_usize() as u64 != TTBR0_EL1.get() {
            log!(
                "Switch page table {:?} -> {:?}",
                TTBR0_EL1.get() as *mut u8,
                self.p4
            );
            asm! {
                "
                    msr	ttbr0_el1, {v}
                    tlbi vmalle1is
                    DSB ISH
                    isb
                ",
                v = in(reg) self.p4.as_usize()
            }
        }

        let exception_frame = {
            if self.exception_frame as usize == 0 {
                let mut frame: *mut ExceptionFrame = (self.kernel_stack_top as usize
                    - ::core::mem::size_of::<ExceptionFrame>())
                    as _;
                (*frame).elr_el1 = self.entry_pc as _;
                (*frame).spsr_el1 = 0b0101;
                frame
            } else {
                let p = self.exception_frame;
                debug_assert!(p as usize != 0);
                self.exception_frame = 0usize as _;
                p
            }
        };
        // if let Some(msg) = self.response_message.take() {
        //     let slot = Address::from((*exception_frame).x2 as *mut Message);
        //     if slot.as_usize() & 0xffff_0000_0000_0000 == 0 {
        //         if super::mm::is_copy_on_write_address(slot) {
        //             super::mm::fix_copy_on_write_address(slot);
        //         }
        //     }
        //     ::core::ptr::write(slot.as_ptr_mut(), msg);
        // }
        if let Some(status) = self.response_status {
            // let slot = Address::from(&(*exception_frame).x0 as *const usize);
            // if slot.as_usize() & 0xffff_0000_0000_0000 == 0 {
            //     if super::mm::is_copy_on_write_address(slot) {
            //         super::mm::fix_copy_on_write_address(slot);
            //     }
            // }
            // slot.store(status);
            (*exception_frame).x0 = ::core::mem::transmute(status);
            self.response_status = None;
        }
        log!("Set SP {:?}", exception_frame);
        log!("Set IP 0x{:?}", (*exception_frame).elr_el1);
        asm!("mov sp, {}", in(reg) exception_frame);
        // debug!(crate::Kernel: "exit_exception ");
        // Return from exception
        super::exception::exit_exception();
    }

    unsafe fn enter_usermode(
        entry: extern "C" fn(_argc: isize, _argv: *const *const u8),
        sp: Address,
        page_table: &mut KernelPageTable,
    ) -> ! {
        log!(
            "TTBR0_EL1={:x} elr_el1={:?} sp_el0={:?}",
            TTBR0_EL1.get(),
            entry as *const extern "C" fn(_argc: isize, _argv: *const *const u8),
            sp
        );
        TargetArch::interrupt().disable();
        asm! {
            "
                msr spsr_el1, {0}
                msr elr_el1, {1}
                msr sp_el0, {2}
                msr	ttbr0_el1, {3}
                tlbi vmalle1is
                DSB ISH
                isb
                eret
            ",
            in(reg) 0usize,
            in(reg) entry,
            in(reg) sp.as_usize(),
            in(reg) page_table as *const _
        }
        unreachable!()
    }
}

impl Drop for AArch64Context {
    fn drop(&mut self) {
        // println!("Context drop");
    }
}

extern "C" {
    #[allow(improper_ctypes)]
    fn switch_context(from: &mut AArch64Context, to: &AArch64Context, p4: usize);
}

global_asm! {"
.global switch_context

switch_context:
    // Store current registers

    mov x8, sp
    str x8, [x0], #8
    stp x19, x20, [x0], #16
    stp x21, x22, [x0], #16
    stp x23, x24, [x0], #16
    stp x25, x26, [x0], #16
    stp x27, x28, [x0], #16
    stp x29, x30, [x0], #16

    stp q0,  q1,  [x0], #32
    stp q2,  q3,  [x0], #32
    stp q4,  q5,  [x0], #32
    stp q6,  q7,  [x0], #32
    stp q8,  q9,  [x0], #32
    stp q10, q11, [x0], #32
    stp q12, q13, [x0], #32
    stp q14, q15, [x0], #32
    stp q16, q17, [x0], #32
    stp q18, q19, [x0], #32
    stp q20, q21, [x0], #32
    stp q22, q23, [x0], #32
    stp q24, q25, [x0], #32
    stp q26, q27, [x0], #32
    stp q28, q29, [x0], #32
    stp q30, q31, [x0], #32

    msr	ttbr0_el1, x2
	tlbi vmalle1is
  	DSB ISH              // ensure completion of TLB invalidation
    isb

    // tlbi vmalle1is
    // DSB SY
    // DMB SY
    // isb
    // msr	ttbr0_el1, x2
    // tlbi vmalle1is
    // DSB SY
    // DMB SY
    // isb

    // Restore registers

    ldr x8, [x1], #8
    mov sp, x8
    ldp x19, x20, [x1], #16
    ldp x21, x22, [x1], #16
    ldp x23, x24, [x1], #16
    ldp x25, x26, [x1], #16
    ldp x27, x28, [x1], #16
    ldp x29, x30, [x1], #16 // FP, SP

    ldp q0,  q1,  [x1], #32
    ldp q2,  q3,  [x1], #32
    ldp q4,  q5,  [x1], #32
    ldp q6,  q7,  [x1], #32
    ldp q8,  q9,  [x1], #32
    ldp q10, q11, [x1], #32
    ldp q12, q13, [x1], #32
    ldp q14, q15, [x1], #32
    ldp q16, q17, [x1], #32
    ldp q18, q19, [x1], #32
    ldp q20, q21, [x1], #32
    ldp q22, q23, [x1], #32
    ldp q24, q25, [x1], #32
    ldp q26, q27, [x1], #32
    ldp q28, q29, [x1], #32
    ldp q30, q31, [x1], #32

    // Return
    ret
"}
