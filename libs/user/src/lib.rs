#![no_std]
#![feature(core_intrinsics)]
#![feature(step_trait)]
#![feature(const_mut_refs)]

extern crate alloc;

mod heap;

pub mod sys;

#[doc(hidden)]
pub mod print;

#[global_allocator]
static ALLOCATOR: heap::UserHeap = heap::UserHeap::new();

#[panic_handler]
fn panic(info: &::core::panic::PanicInfo) -> ! {
    println!("{}", info);
    sys::exit()
}
