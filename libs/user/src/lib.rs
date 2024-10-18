#![no_std]
#![feature(step_trait)]

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
