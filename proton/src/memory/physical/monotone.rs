use core::{iter::Step, ops::Range};

use crate::utils::page::*;

use super::PhysicalPageResource;

pub struct Monotone {
    cursor: Option<Frame>,
    limit: Option<Frame>,
    range_index: usize,
    all_frames: &'static [Range<Frame>],
}

impl Monotone {
    pub const fn new() -> Self {
        Self {
            cursor: None,
            limit: None,
            range_index: 0,
            all_frames: &[],
        }
    }

    #[cold]
    fn acquire_slow<S: PageSize>(&mut self, count: usize) -> Result<Range<Frame<S>>, ()> {
        self.range_index += 1;
        let range = self.all_frames.get(self.range_index).ok_or(())?.clone();
        self.cursor = Some(range.start);
        self.limit = Some(range.end);
        self.acquire(count)
    }
}

impl PhysicalPageResource for Monotone {
    fn init(&mut self, frames: &'static [Range<Frame>]) {
        let first_range = frames[0].clone();
        self.cursor = Some(first_range.start);
        self.limit = Some(first_range.end);
        self.range_index = 0;
        self.all_frames = frames;
    }

    #[inline(always)]
    fn acquire<S: PageSize>(&mut self, count: usize) -> Result<Range<Frame<S>>, ()> {
        if let (Some(cursor), Some(limit)) = (self.cursor, self.limit) {
            let aligned_start = Frame::<S>::new(cursor.start().align_up(S::BYTES));
            let end = Step::forward(aligned_start, count);
            if end.start() <= limit.start() {
                self.cursor = Some(Frame::new(end.start()));
                return Ok(aligned_start..end);
            } else {
                self.acquire_slow(count)
            }
        } else {
            self.acquire_slow(count)
        }
    }

    #[inline(always)]
    fn release<S: PageSize>(&mut self, _frames: Range<Frame<S>>) -> Result<(), ()> {
        Ok(())
    }
}
