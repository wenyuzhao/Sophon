use alloc::boxed::Box;
use core::iter::Step;
use crate::mm::*;
use crate::mm::heap_constants::*;
use crate::exception::ExceptionFrame;

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
    pub p4: Frame,
    pub kernel_stack: Option<Box<KernelStack>>,
}

impl Context {
    pub const fn empty() -> Self {
        Self {
            x19: 0, x20: 0, x21: 0, x22: 0, x23: 0, x24: 0,
            x25: 0, x26: 0, x27: 0, x28: 0, x29: 0,
            pc: 0 as _, sp: 0 as _,
            p4: Frame::ZERO,
            kernel_stack: None,
        }
    }

    /// Create a new context with empty regs, given kernel stack,
    /// and current p4 table
    pub fn new(entry: *const extern fn() -> !) -> Self {
        // Alloc page table
        let p4 = unsafe {
            let p4_frame = frame_allocator::alloc::<Size4K>().unwrap();
            let p4_page = crate::mm::map_kernel_temporarily(p4_frame, PageFlags::_PAGE_TABLE_FLAGS, None);
            let p4 = p4_page.start().as_ref_mut::<PageTable<L4>>();
            p4.entries[511].set(p4_frame, PageFlags::_PAGE_TABLE_FLAGS);
            p4_frame
        };
        // Alloc kernel stack
        let kernel_stack = KernelStack::new();
        let sp: *mut u8 = kernel_stack.end_address().as_ptr_mut();
        Self {
            x19: 0, x20: 0, x21: 0, x22: 0, x23: 0, x24: 0,
            x25: 0, x26: 0, x27: 0, x28: 0, x29: 0,
            pc: unsafe { entry as _ },
            sp, p4,
            kernel_stack: Some(kernel_stack),
        }
    }
 
    pub fn fork(&self, parent_sp: usize, child_return_value: usize) -> Self {
        let mut ctx = Context {
            x19: self.x19, x20: self.x20, x21: self.x21, x22: self.x22,
            x23: self.x23, x24: self.x24, x25: self.x25, x26: self.x26,
            x27: self.x27, x28: self.x28, x29: self.x29,
            sp: self.sp, pc: self.pc, p4: self.p4,
            kernel_stack: Some({
                let mut kernel_stack = KernelStack::new();
                kernel_stack.copy_from(self.kernel_stack.as_ref().unwrap());
                kernel_stack
            }),
        };
        ctx.sp = {
            let sp_offset = parent_sp - self.kernel_stack.as_ref().unwrap().start_address().as_usize();
            (ctx.kernel_stack.as_ref().unwrap().start_address() + sp_offset).as_ptr_mut()
        };
        ctx.pc = crate::exception::exit_from_exception as _;
        ctx.p4 = paging::fork_page_table(self.p4);
        // Set child process return value
        {
            let sp_offset = parent_sp - self.kernel_stack.as_ref().unwrap().start_address().as_usize();
            let child_exception_frame_ptr = ctx.kernel_stack.as_ref().unwrap().start_address() + sp_offset;
            let child_exception_frame = unsafe { child_exception_frame_ptr.as_ref_mut::<ExceptionFrame>() };
            child_exception_frame.x0 = 0;
        }
        ctx
    }

    pub unsafe extern fn switch_to(&mut self, ctx: &Context) {
        // if self.p4 != ctx.p4 {
            // println!("Switch P4: {:?} -> {:?}", self.p4, ctx.p4);
            // println!("Switch SP: {:?} -> {:?}", self.sp, ctx.sp);
            // asm! {"
            //     msr	ttbr0_el1, $0
            //     tlbi vmalle1is
            //     DSB ISH
            //     isb
            // "::"r"(ctx.p4.start().as_usize())}
        // }
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

    // Return
    ret
"}