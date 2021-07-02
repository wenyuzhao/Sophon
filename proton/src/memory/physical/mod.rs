use core::ops::Range;

use crate::utils::page::*;
use spin::Mutex;

pub trait PhysicalPageResource: Sized {
    fn init(&mut self, frames: &'static [Range<Frame>]);
    fn acquire<S: PageSize>(&mut self, count: usize) -> Result<Range<Frame<S>>, ()>;
    fn release<S: PageSize>(&mut self, frames: Range<Frame<S>>) -> Result<(), ()>;
}

mod monotone;

pub static PHYSICAL_PAGE_RESOURCE: Mutex<impl PhysicalPageResource> =
    Mutex::new(monotone::Monotone::new());
