use core::alloc::GlobalAlloc;
use core::alloc::Layout;

pub struct KernelModuleAllocator;

unsafe impl GlobalAlloc for KernelModuleAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        super::SERVICE.alloc(layout).unwrap().as_mut_ptr()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        super::SERVICE.dealloc(ptr.into(), layout)
    }
}
