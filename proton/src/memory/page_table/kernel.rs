use super::*;
use crate::arch::{Arch, ArchInterrupt, TargetArch};
use crate::memory::physical::{PhysicalPageResource, PHYSICAL_PAGE_RESOURCE};
use crate::utils::address::*;
use crate::utils::page::*;
use core::fmt::Debug;
use core::intrinsics::transmute;
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};
use core::ops::{Index, IndexMut};

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
    pub fn alloc() -> &'static mut Self {
        let frame = Self::alloc_frame4k();
        unsafe {
            frame.zero();
            frame.start().as_mut()
        }
    }

    #[inline]
    pub fn get() -> &'static mut Self {
        unsafe { TargetArch::get_current_page_table().start().as_mut() }
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
                    TargetArch::set_current_page_table(self.old);
                }
                if self.irq_enabled {
                    <TargetArch as Arch>::Interrupt::enable();
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
            old: TargetArch::get_current_page_table(),
            new: Frame::new((self as *const _ as usize).into()),
            irq_enabled: <TargetArch as Arch>::Interrupt::is_enabled(),
        };
        if x.irq_enabled {
            <TargetArch as Arch>::Interrupt::disable();
        }
        if x.old != x.new {
            TargetArch::set_current_page_table(x.new);
        }
        x

        // struct PageTables {
        //     old: Frame,
        //     new: Frame,
        // }
        // impl Drop for PageTables {
        //     fn drop(&mut self) {
        //         if self.old != self.new {
        //             TargetArch::set_current_page_table(self.old);
        //         }
        //     }
        // }
        // impl Deref for PageTables {
        //     type Target = PageTable;
        //     fn deref(&self) -> &Self::Target {
        //         unsafe { self.new.start().as_ref() }
        //     }
        // }
        // impl DerefMut for PageTables {
        //     fn deref_mut(&mut self) -> &mut Self::Target {
        //         unsafe { self.new.start().as_mut() }
        //     }
        // }
        // let x = PageTables {
        //     old: TargetArch::get_current_page_table(),
        //     new: Frame::new((self as *const _ as usize).into()),
        // };
        // if x.old != x.new {
        //     TargetArch::set_current_page_table(x.new);
        // }
        // x
    }

    pub fn identity_map<S: PageSize>(&mut self, frame: Frame<S>, flags: PageFlags) -> Page<S> {
        self.map(Page::new(frame.start().as_usize().into()), frame, flags)
    }

    fn alloc_frame4k() -> Frame<Size4K> {
        let frame = PHYSICAL_PAGE_RESOURCE
            .lock()
            .acquire::<Size4K>(1)
            .unwrap()
            .start;
        unsafe {
            frame.zero();
        }
        frame
    }

    pub fn map<S: PageSize>(
        &mut self,
        page: Page<S>,
        frame: Frame<S>,
        flags: PageFlags,
    ) -> Page<S> {
        // P4
        let table = self;
        // P3
        let index = PageTable::<L4>::get_index(page.start());
        if table[index].is_empty() {
            table[index].set(Self::alloc_frame4k(), PageFlags::page_table_flags());
        }
        let table = table.get_next_table(index).unwrap();
        if S::BYTES == Size1G::BYTES {
            table.entries[PageTable::<L3>::get_index(page.start())].set(frame, flags);
            return page;
        }
        // P2
        let index = PageTable::<L3>::get_index(page.start());
        if table.entries[index].is_empty() {
            table.entries[index].set(Self::alloc_frame4k(), PageFlags::page_table_flags());
        }
        let table = table.get_next_table(index).unwrap();
        if S::BYTES == Size2M::BYTES {
            table.entries[PageTable::<L2>::get_index(page.start())].set(frame, flags);
            return page;
        }
        // P1
        let index = PageTable::<L2>::get_index(page.start());
        if table.entries[index].is_empty() {
            table.entries[index].set(Self::alloc_frame4k(), PageFlags::page_table_flags());
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
