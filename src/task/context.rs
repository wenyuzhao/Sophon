use alloc::boxed::Box;
use core::iter::Step;
use crate::mm::*;
use crate::mm::heap_constants::*;
use crate::exception::ExceptionFrame;
use cortex_a::regs::*;

#[repr(C, align(4096))]
pub struct KernelStack {
    /// This page is protected to trap stack overflow
    guard: [u8; Size4K::SIZE],
    stack: [u8; KERNEL_STACK_SIZE],
}

impl KernelStack {
    pub fn new() -> Box<Self> {
        let kernel_stack = unsafe { Box::<KernelStack>::new_uninit().assume_init() };
        kernel_stack.init();
        kernel_stack
    }
    pub fn init(&self) {
        let guard_page = Page::<Size4K, V>::new(Address::from(&self.guard as *const [u8; Size4K::SIZE]));
        update_kernel_page_flags(guard_page, PageFlags::_KERNEL_STACK_GUARD_FLAGS);
        let stack_page_start = Page::<Size4K, V>::new(Address::from(&self.stack as *const [u8; KERNEL_STACK_SIZE]));
        let stack_page_end = stack_page_start.add_usize(KERNEL_STACK_PAGES).unwrap();
        for stack_page in stack_page_start..stack_page_end {
            update_kernel_page_flags(stack_page, PageFlags::_KERNEL_STACK_FLAGS);
        }
    }
    pub fn start_address(&self) -> Address {
        let stack_start = Address::from(&self.stack as *const [u8; KERNEL_STACK_SIZE]);
        stack_start
    }
    pub fn end_address(&self) -> Address {
        let stack_start = Address::from(&self.stack as *const [u8; KERNEL_STACK_SIZE]);
        stack_start + KERNEL_STACK_SIZE
    }
    pub fn copy_from(&mut self, other: &Self) {
        for i in 0..KERNEL_STACK_SIZE {
            self.stack[i] = other.stack[i];
        }
    }
}

impl Drop for KernelStack {
    // Unprotect stack pages
    fn drop(&mut self) {
        let guard_page = Page::<Size4K, V>::new(Address::from(&self.guard as *const [u8; Size4K::SIZE]));
        update_kernel_page_flags(guard_page, PageFlags::_KERNEL_DATA_FLAGS_4K);
        let stack_page_start = Page::<Size4K, V>::new(Address::from(&self.stack as *const [u8; KERNEL_STACK_SIZE]));
        let stack_page_end = stack_page_start.add_usize(KERNEL_STACK_PAGES).unwrap();
        for stack_page in stack_page_start..stack_page_end {
            update_kernel_page_flags(stack_page, PageFlags::_KERNEL_DATA_FLAGS_4K);
        }
    }
}

/// Represents the archtectural context (i.e. registers)
#[repr(C)]
pub struct Context {
    pub sp: *mut u8,
    pub x19: usize,
    pub x20: usize,
    pub x21: usize,
    pub x22: usize,
    pub x23: usize,
    pub x24: usize,
    pub x25: usize,
    pub x26: usize,
    pub x27: usize,
    pub x28: usize,
    pub x29: usize, // FP
    pub pc: *mut u8,  // x30
    
    pub q0:  u128, pub q1:  u128, pub q2:  u128, pub q3:  u128, pub q4:  u128, pub q5:  u128, pub q6:  u128, pub q7:  u128,
    pub q8:  u128, pub q9:  u128, pub q10: u128, pub q11: u128, pub q12: u128, pub q13: u128, pub q14: u128, pub q15: u128,
    pub q16: u128, pub q17: u128, pub q18: u128, pub q19: u128, pub q20: u128, pub q21: u128, pub q22: u128, pub q23: u128,
    pub q24: u128, pub q25: u128, pub q26: u128, pub q27: u128, pub q28: u128, pub q29: u128, pub q30: u128, pub q31: u128,

    pub p4: Frame,
    pub kernel_stack: Option<Box<KernelStack>>,
    pub exception_frame: *mut ExceptionFrame,
}

impl Context {
    pub fn empty() -> Self {
        let mut c: Self = unsafe { ::core::mem::zeroed() };
        // c.p4 = Frame::new((TTBR0_EL1.get() as usize).into());
        // println!("p4 {:?}", c.p4);
        c
    }

    /// Create a new context with empty regs, given kernel stack,
    /// and current p4 table
    pub fn new(entry: *const extern fn() -> !) -> Self {
        // Alloc page table
        let p4 = unsafe {
            let p4_frame = frame_allocator::alloc::<Size4K>().unwrap();
            let p4_page = crate::mm::map_kernel_temporarily(p4_frame, PageFlags::_PAGE_TABLE_FLAGS, None);
            let p4 = p4_page.start().as_ref_mut::<PageTable<L4>>();
            for i in 0..512 {
                p4.entries[i].clear();
            }
            p4.entries[511].set(p4_frame, PageFlags::_PAGE_TABLE_FLAGS);
            p4_frame
        };
        // Alloc kernel stack
        let kernel_stack = KernelStack::new();
        let sp: *mut u8 = kernel_stack.end_address().as_ptr_mut();
        let mut ctx = Self::empty();
        ctx.pc = unsafe { entry as _ };
        ctx.sp = sp;
        ctx.p4 = p4;
        ctx.kernel_stack = Some(kernel_stack);
        ctx
    }
 
    pub fn fork(&self) -> Self {
        let mut ctx = Context {
            x19: self.x19, x20: self.x20, x21: self.x21, x22: self.x22,
            x23: self.x23, x24: self.x24, x25: self.x25, x26: self.x26,
            x27: self.x27, x28: self.x28, x29: self.x29,
            sp: self.sp, pc: self.pc, p4: self.p4,
            q0:  self.q0,  q1:  self.q1,  q2:  self.q2,  q3:  self.q3,  q4:  self.q4,  q5:  self.q5,  q6:  self.q6,  q7:  self.q7,
            q8:  self.q8,  q9:  self.q9,  q10: self.q10, q11: self.q11, q12: self.q12, q13: self.q13, q14: self.q14, q15: self.q15,
            q16: self.q16, q17: self.q17, q18: self.q18, q19: self.q19, q20: self.q20, q21: self.q21, q22: self.q22, q23: self.q23,
            q24: self.q24, q25: self.q25, q26: self.q26, q27: self.q27, q28: self.q28, q29: self.q29, q30: self.q30, q31: self.q31,
            exception_frame: 0usize as _,
            kernel_stack: Some({
                let mut kernel_stack = KernelStack::new();
                kernel_stack.copy_from(self.kernel_stack.as_ref().unwrap());
                kernel_stack
            }),
        };
        ctx.exception_frame = {
            let ef_offset = self.exception_frame as usize - self.kernel_stack.as_ref().unwrap().start_address().as_usize();
            (ctx.kernel_stack.as_ref().unwrap().start_address() + ef_offset).as_ptr_mut()
        };
        ctx.sp = {
            println!("Fork, sp = {:?}, kstack = {:?}", self.sp, self.kernel_stack.as_ref().unwrap().start_address());
            let sp_offset = self.sp as usize - self.kernel_stack.as_ref().unwrap().start_address().as_usize();
            (ctx.kernel_stack.as_ref().unwrap().start_address() + sp_offset).as_ptr_mut()
        };
        ctx.p4 = paging::fork_page_table(self.p4);
        // Set parent/child process return value
        unsafe {
            (*self.exception_frame).x0 = 0;
            (*ctx.exception_frame).x0 = 0;
        }
        ctx
    }

    pub unsafe extern fn switch_to(&mut self, ctx: &Context) {
        switch_context(self, ctx, ctx.p4.start().as_usize())
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        println!("Context drop");
    }
}

extern {
    fn switch_context(from: &mut Context, to: &Context, p4: usize);
    pub fn start_task();
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

    tlbi vmalle1is
    DSB ISH
    isb
    msr	ttbr0_el1, x2
    tlbi vmalle1is
    DSB ISH
    isb

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