use super::*;
use crate::address::*;
use crate::page::*;
use core::fmt::Debug;
use core::intrinsics::transmute;
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};
use core::ops::{Index, IndexMut};
#[cfg(target_arch = "aarch64")]
use cortex_a::registers::TTBR0_EL1;
#[cfg(target_arch = "aarch64")]
use tock_registers::interfaces::Readable;

#[repr(C, align(4096))]
#[derive(Debug)]
pub struct PageTable<L: TableLevel = L4> {
    pub entries: [PageTableEntry; 512],
    phantom: PhantomData<L>,
}

impl<L: TableLevel> Index<usize> for PageTable<L> {
    type Output = PageTableEntry;

    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}

impl<L: TableLevel> IndexMut<usize> for PageTable<L> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.entries[index]
    }
}

impl<L: TableLevel> PageTable<L> {
    const MASK: usize = 0b111111111 << L::SHIFT;

    #[inline]
    pub fn is_clear(&self) -> bool {
        self.entries.iter().all(|e| e.is_empty())
    }

    #[inline]
    pub const fn get_index(a: Address<V>) -> usize {
        (a.as_usize() & Self::MASK) >> L::SHIFT
    }

    fn get_next_table(&mut self, index: usize) -> Option<&'static mut PageTable<L::NextLevel>> {
        if self.entries[index].present() && !self.entries[index].is_block() {
            let addr = self.entries[index].address();
            Some(unsafe { transmute(addr) })
        } else {
            None
        }
    }

    fn walk_mut_impl(
        &mut self,
        base: Address<V>,
        visitor: &mut impl FnMut(&mut PageTableEntry, Address<V>, usize),
    ) {
        for (i, entry) in self.entries.iter_mut().enumerate() {
            let addr = base + (i << L::SHIFT);
            if !entry.present() {
                continue;
            }
            if L::ID == L1::ID {
                visitor(entry, addr, L::ID - 1);
            } else {
                if entry.is_block() {
                    visitor(entry, addr, L::ID - 1);
                } else {
                    let phy_addr = entry.address();
                    let next_table: &'static mut PageTable<L::NextLevel> =
                        unsafe { transmute(phy_addr) };
                    next_table.walk_mut_impl(addr, visitor);
                }
            }
        }
    }

    pub fn walk_mut(&mut self, visitor: &mut impl FnMut(&mut PageTableEntry, Address<V>, usize)) {
        self.walk_mut_impl(Address::ZERO, visitor);
    }

    pub fn clone(&self, pa: &impl PageAllocator<P>) -> &'static mut Self {
        let frame = PageTable::<L4>::alloc_frame4k(pa);
        unsafe {
            frame.zero();
            let new_table = frame.start().as_mut::<PageTable<L>>();
            for (i, entry) in self.entries.iter().enumerate() {
                new_table.entries[i] = entry.clone();
                if L::ID == 4 && i >= 480 {
                    // This are kernel page tables, don't duplicate them
                    continue;
                }
                if L::ID != L1::ID
                    && entry.present()
                    && !entry.is_block()
                    && entry.flags().contains(PageFlags::USER)
                {
                    let next_table = entry.address().as_mut::<PageTable<L::NextLevel>>();
                    let next_table_cloned = next_table.clone(pa);
                    new_table.entries[i].set::<Size4K>(
                        Frame::new(Address::from(
                            next_table_cloned as *mut PageTable<L::NextLevel> as usize,
                        )),
                        entry.flags(),
                    );
                }
            }
            new_table
        }
    }

    pub fn release(&mut self, pa: &impl PageAllocator<P>) {
        for (i, entry) in self.entries.iter_mut().enumerate() {
            if L::ID == L4::ID && i >= 480 {
                // Kernel page table entries are shared. Don't release them.
                break;
            }
            if entry.is_empty() || !entry.present() {
                continue;
            }
            if entry.is_block() || L::ID == L1::ID {
                // Contains a frame
                let page = entry.address();
                match L::ID {
                    L1::ID => pa.dealloc::<Size4K>(Frame::new(page)),
                    L2::ID => pa.dealloc::<Size2M>(Frame::new(page)),
                    L3::ID => pa.dealloc::<Size1G>(Frame::new(page)),
                    _ => unreachable!(),
                }
            } else {
                // Contains a pointer to the next level page table
                let next_pt_addr = entry.address();
                let next_page_table = unsafe { next_pt_addr.as_mut::<PageTable<L::NextLevel>>() };
                next_page_table.release(pa);
            }
            entry.clear();
        }
        pa.dealloc::<Size4K>(Frame::new(Address::from(self)));
    }
}

impl PageTable<L4> {
    pub fn alloc(pa: &impl PageAllocator<P>) -> &'static mut Self {
        let frame = Self::alloc_frame4k(pa);
        unsafe {
            frame.zero();
            frame.start().as_mut()
        }
    }

    fn alloc_frame4k(pa: &impl PageAllocator<P>) -> Frame<Size4K> {
        let frame = pa.alloc::<Size4K>().unwrap();
        unsafe {
            frame.zero();
        }
        frame
    }

    #[inline]
    #[cfg(target_arch = "x86_64")]
    pub fn get() -> &'static mut Self {
        unimplemented!()
    }

    #[inline]
    #[cfg(target_arch = "aarch64")]
    pub fn get() -> &'static mut Self {
        unsafe { &mut *(TTBR0_EL1.get() as usize as *mut Self) }
    }

    #[inline]
    #[cfg(target_arch = "x86_64")]
    pub fn set(_p4: *mut Self) {
        unimplemented!()
    }

    #[inline]
    #[cfg(target_arch = "aarch64")]
    pub fn set(p4: *mut Self) {
        use core::arch::asm;

        unsafe {
            asm! {
                "
                msr	ttbr0_el1, {v}
                tlbi vmalle1is
                DSB ISH
                isb
            ",
                v = in(reg) p4
            }
        }
    }

    #[inline]
    pub fn enable_temporarily(&self) -> impl Drop + DerefMut + Deref<Target = PageTable> {
        struct PageTables {
            old: Frame,
            new: Frame,
            irq_enabled: bool,
        }
        impl Drop for PageTables {
            fn drop(&mut self) {
                PageTable::<L4>::set(self.old.start().as_mut_ptr());
                if self.irq_enabled {
                    interrupt::enable();
                }
            }
        }
        impl Deref for PageTables {
            type Target = PageTable;
            fn deref(&self) -> &Self::Target {
                unsafe { self.new.start().as_ref() }
            }
        }
        impl DerefMut for PageTables {
            fn deref_mut(&mut self) -> &mut Self::Target {
                unsafe { self.new.start().as_mut() }
            }
        }
        let x = PageTables {
            old: Page::new(PageTable::<L4>::get().into()),
            new: Frame::new((self as *const _ as usize).into()),
            irq_enabled: interrupt::is_enabled(),
        };
        if x.irq_enabled {
            interrupt::disable();
        }
        Self::set(x.new.start().as_mut_ptr());
        x
    }

    pub fn translate_with_flags(
        &mut self,
        a: Address<V>,
    ) -> Option<(Address<P>, PageFlagSet, usize)> {
        // P4
        let table = self;
        // P3
        let index = PageTable::<L4>::get_index(a);
        if table[index].is_empty() {
            return None;
        }
        let table = table.get_next_table(index).unwrap();
        let index = PageTable::<L3>::get_index(a);
        if table[index].is_empty() {
            return None;
        }
        if table[index].is_block() {
            return Some((
                table[index].address() + (a.as_usize() & Page::<Size1G>::MASK),
                table[index].flags(),
                3,
            ));
        }
        // P2
        let table = table.get_next_table(index).unwrap();
        let index = PageTable::<L2>::get_index(a);
        if table[index].is_empty() {
            return None;
        }
        if table[index].is_block() {
            return Some((
                table[index].address() + (a.as_usize() & Page::<Size2M>::MASK),
                table[index].flags(),
                2,
            ));
        }
        // P1
        let table = table.get_next_table(index).unwrap();
        let index = PageTable::<L1>::get_index(a);
        if table[index].is_empty() {
            return None;
        } else {
            return Some((
                table[index].address() + (a.as_usize() & Page::<Size4K>::MASK),
                table[index].flags(),
                1,
            ));
        }
    }

    pub fn translate(&mut self, a: Address<V>) -> Option<Address<P>> {
        self.translate_with_flags(a).map(|(p, _, _)| p)
    }

    pub fn identity_map<S: PageSize>(
        &mut self,
        frame: Frame<S>,
        flags: PageFlagSet,
        pa: &impl PageAllocator<P>,
    ) -> Page<S> {
        self.map(Page::new(frame.start().as_usize().into()), frame, flags, pa)
    }

    pub fn map<S: PageSize>(
        &mut self,
        page: Page<S>,
        frame: Frame<S>,
        flags: PageFlagSet,
        pa: &impl PageAllocator<P>,
    ) -> Page<S> {
        // P4
        let table = self;
        // P3
        let index = PageTable::<L4>::get_index(page.start());
        if table[index].is_empty() {
            table[index].set(Self::alloc_frame4k(pa), PageFlags::page_table_flags());
        }
        let table = table.get_next_table(index).unwrap();
        if S::BYTES == Size1G::BYTES {
            table.entries[PageTable::<L3>::get_index(page.start())].set(frame, flags);
            return page;
        }
        // P2
        let index = PageTable::<L3>::get_index(page.start());
        if table.entries[index].is_empty() {
            table.entries[index].set(Self::alloc_frame4k(pa), PageFlags::page_table_flags());
        }
        let table = table.get_next_table(index).unwrap();
        if S::BYTES == Size2M::BYTES {
            table.entries[PageTable::<L2>::get_index(page.start())].set(frame, flags);
            return page;
        }
        // P1
        let index = PageTable::<L2>::get_index(page.start());
        if table.entries[index].is_empty() {
            table.entries[index].set(Self::alloc_frame4k(pa), PageFlags::page_table_flags());
        }
        let table = table.get_next_table(index).unwrap();
        table.entries[PageTable::<L1>::get_index(page.start())]
            .set(frame, flags | PageFlags::SMALL_PAGE);
        page
    }

    pub fn unmap<S: PageSize>(&mut self, page: Page<S>, pa: &impl PageAllocator<P>) {
        let a = page.start();
        // P4
        let p4 = self;
        // P3
        let p4_index = PageTable::<L4>::get_index(a);
        debug_assert!(!p4[p4_index].is_empty());
        let p3 = p4.get_next_table(p4_index).unwrap();
        let p3_index = PageTable::<L3>::get_index(a);
        debug_assert!(!p3[p3_index].is_empty());
        if !p3[p3_index].is_block() {
            // P2
            let p2 = p3.get_next_table(p3_index).unwrap();
            let p2_index = PageTable::<L2>::get_index(a);
            debug_assert!(!p2[p2_index].is_empty());
            if !p2[p2_index].is_block() {
                // P1
                let p1 = p2.get_next_table(p2_index).unwrap();
                let p1_index = PageTable::<L1>::get_index(a);
                // Clear P1 entry
                p1[p1_index].clear();
                if !p1.is_clear() {
                    return;
                }
                // Release P1
                pa.dealloc::<S>(Page::new(p1.into()));
            }
            // Clear P2 entry
            p2[p2_index].clear();
            if !p2.is_clear() {
                return;
            }
            // Release P2
            pa.dealloc::<S>(Page::new(p2.into()));
        }
        // Clear P3 entry
        p3[p3_index].clear();
        if !p3.is_clear() {
            return;
        }
        // Release P3
        pa.dealloc::<S>(Page::new(p3.into()));
        // Clear P4 entry
        p4[p4_index].clear();
    }

    pub fn copy_on_write<S: PageSize>(&mut self, page: Page<S>, pa: &impl PageAllocator<P>) {
        let (a, mut flags, _) = self.translate_with_flags(page.start()).unwrap();
        flags = flags - PageFlags::NO_WRITE;
        flags = flags - PageFlags::COPY_ON_WRITE;
        let b = pa.alloc::<S>().unwrap();
        unsafe {
            core::ptr::copy_nonoverlapping::<u8>(a.as_ptr(), b.start().as_mut_ptr(), S::BYTES);
        }
        self.map(page, b, flags, pa);
    }
}
