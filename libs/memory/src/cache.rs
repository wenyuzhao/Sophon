use core::ops::Range;

use cortex_a::asm::barrier::{dsb, isb, SY};

use crate::address::{Address, MemoryKind};

pub fn flush_cache<K: MemoryKind>(range: Range<Address<K>>) {
    const CACHE_LINE_SIZE: usize = 64;
    let start = range.start.align_down(CACHE_LINE_SIZE);
    let end = if range.end.is_aligned_to(CACHE_LINE_SIZE) {
        range.end
    } else {
        range.end.align_up(CACHE_LINE_SIZE)
    };
    unsafe {
        dsb(SY);
        isb(SY);
        for cache_line in (start..end).step_by(CACHE_LINE_SIZE) {
            asm!(
                "
                    dc cvau, x0
                    ic ivau, x0
                ",
                in("x0") cache_line.as_usize(),
            );
        }
        dsb(SY);
        isb(SY);
    }
}
