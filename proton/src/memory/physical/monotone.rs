use core::ops::Range;

use crate::utils::page::*;

use super::PhysicalPageResource;

pub struct Monotone {
    cursor: Frame,
    limit: Frame,
    range_index: usize,
    all_frames: &'static [Range<Frame>],
}

impl Monotone {
    pub const fn new() -> Self {
        Self {
            cursor: Frame::ZERO,
            limit: Frame::ZERO,
            range_index: 0,
            all_frames: &[],
        }
    }

    #[cold]
    fn acquire_slow<S: PageSize>(&mut self, count: usize) -> Result<Range<Frame<S>>, ()> {
        self.range_index += 1;
        let range = self.all_frames.get(self.range_index).ok_or(())?.clone();
        self.cursor = range.start;
        self.limit = range.end;
        self.acquire(count)
    }
}

impl PhysicalPageResource for Monotone {
    fn init(&mut self, frames: &'static [Range<Frame>]) {
        let first_range = frames[0].clone();
        self.cursor = first_range.start;
        self.limit = first_range.end;
        self.range_index = 0;
        self.all_frames = frames;
    }

    #[inline(always)]
    fn acquire<S: PageSize>(&mut self, count: usize) -> Result<Range<Frame<S>>, ()> {
        let aligned_start = Frame::<S>::new(Frame::<S>::align_up(self.cursor.start()));
        let end = aligned_start + count;
        if end.start() <= self.limit.start() {
            self.cursor = Frame::new(end.start());
            return Ok(aligned_start..end);
        } else {
            self.acquire_slow(count)
        }
    }

    #[inline(always)]
    fn release<S: PageSize>(&mut self, _frames: Range<Frame<S>>) -> Result<(), ()> {
        Ok(())
    }
}
