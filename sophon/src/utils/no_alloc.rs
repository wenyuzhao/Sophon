use core::alloc::{GlobalAlloc, Layout};

pub struct NoAlloc;

unsafe impl GlobalAlloc for NoAlloc {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        unreachable!()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        unreachable!()
    }
}
