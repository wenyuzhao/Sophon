use super::FrameAllocator;
use crate::utils::address::*;
use crate::utils::page::*;

pub struct BumpFrameAllocator {
    start: Address<P>,
    cursur: Address<P>,
    limit: Address<P>,
}

impl BumpFrameAllocator {
    pub const fn new((start, limit): (Address<P>, Address<P>)) -> Self {
        Self {
            start,
            limit,
            cursur: start,
        }
    }
}

impl FrameAllocator for BumpFrameAllocator {
    fn identity_alloc<S: PageSize>(&mut self, p: Frame<S>) {
        let start = p.start();
        let end = p.start() + Frame::<S>::SIZE;
        assert!(
            start.as_usize() < self.start.as_usize() || end.as_usize() >= self.limit.as_usize()
        );
    }

    fn alloc<S: PageSize>(&mut self) -> Frame<S> {
        let result = Frame::<S>::align_up(self.cursur);
        self.cursur = result + Frame::<S>::SIZE;
        assert!(self.cursur.as_usize() <= self.limit.as_usize());
        return Frame::new(result);
    }

    fn free<S: PageSize>(&mut self, _: Frame<S>) {
        // do nothing
    }
}
