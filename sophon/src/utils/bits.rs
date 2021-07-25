use core::sync::atomic::{AtomicUsize, Ordering};

#[derive(PartialEq, Eq)]
pub struct BitField {
    pub bits: usize,
    pub shift: usize,
}

pub trait BitFieldSlot {
    fn get_bool<const BITS: BitField>(&self) -> bool {
        self.get::<BITS>() != 0
    }
    fn get<const BITS: BitField>(&self) -> usize;
    fn set<const BITS: BitField>(&self, value: usize);
}

impl BitFieldSlot for AtomicUsize {
    #[inline(always)]
    fn get<const BITS: BitField>(&self) -> usize {
        let value = self.load(Ordering::Relaxed);
        (value >> BITS.shift) & ((1usize << BITS.bits) - 1)
    }

    #[inline(always)]
    fn set<const BITS: BitField>(&self, value: usize) {
        let old_value = self.load(Ordering::Relaxed);
        let mask = ((1usize << BITS.bits) - 1) << BITS.shift;
        let shifted_value = value << BITS.shift;
        debug_assert!((shifted_value & !mask) == 0);
        let new_value = (old_value & !mask) | (value << BITS.shift);
        self.store(new_value, Ordering::Relaxed);
    }
}
