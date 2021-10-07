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
    entries: [PageTableEntry; 512],
    phantom: PhantomData<L>,
}

impl<L: TableLevel> const Index<usize> for PageTable<L> {
    type Output = PageTableEntry;

    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}

impl<L: TableLevel> const IndexMut<usize> for PageTable<L> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.entries[index]
    }
}

impl<L: TableLevel> PageTable<L> {
    const MASK: usize = 0b111111111 << L::SHIFT;

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
    #[cfg(target_arch = "aarch64")]
    pub fn get() -> &'static mut Self {
        unsafe { &mut *(TTBR0_EL1.get() as usize as *mut Self) }
    }

    #[inline]
    #[cfg(target_arch = "aarch64")]
    pub fn set(p4: *mut Self) {
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
                if self.old != self.new {
                    PageTable::<L4>::set(self.old.start().as_mut_ptr());
                }
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
        if x.old != x.new {
            Self::set(x.new.start().as_mut_ptr());
        }
        x
    }

    pub fn identity_map<S: PageSize>(
        &mut self,
        frame: Frame<S>,
        flags: PageFlags,
        pa: &impl PageAllocator<P>,
    ) -> Page<S> {
        self.map(Page::new(frame.start().as_usize().into()), frame, flags, pa)
    }

    pub fn map<S: PageSize>(
        &mut self,
        page: Page<S>,
        frame: Frame<S>,
        flags: PageFlags,
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
            .set(frame, flags | PageFlag::SMALL_PAGE);
        page
    }

    pub fn unmap<S: PageSize>(&mut self, _page: Page<S>) {
        unimplemented!()
    }
}
