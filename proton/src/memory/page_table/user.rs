use super::*;
use crate::memory::physical::*;
use crate::utils::address::*;
use crate::utils::page::*;
use core::ops::Deref;
use core::{fmt::Debug, marker::PhantomData};

#[repr(C, align(4096))]
#[derive(Debug)]
pub struct UserPageTable<L: TableLevel> {
    pub entries: [PageTableEntry; 512],
    phantom: PhantomData<L>,
}

impl<L: TableLevel> UserPageTable<L> {
    const MASK: usize = 0b111111111 << L::SHIFT;

    fn zero(&mut self) {
        for i in 0..512 {
            unsafe {
                ::core::intrinsics::volatile_store(&mut self.entries[i], PageTableEntry(0));
            }
        }
    }

    #[inline]
    pub fn get_index(a: Address<V>) -> usize {
        (a.as_usize() & Self::MASK) >> L::SHIFT
    }

    fn next_table_address(&self, index: usize) -> Option<usize> {
        debug_assert!(L::ID > 1);
        debug_assert!(index < 512);
        if self.entries[index].present() && !self.entries[index].is_block() {
            let table_address = self as *const _ as usize;
            let mut a = (table_address << 9) | (index << 12);
            if self as *const _ as usize & (0xffff << 48) == 0 {
                a &= 0x0000_ffff_ffff_ffff;
            }
            Some(a)
        } else {
            None
        }
    }

    pub fn next_table(&self, index: usize) -> Option<&'static mut UserPageTable<L::NextLevel>> {
        debug_assert!(L::ID > 1);
        if let Some(address) = self.next_table_address(index) {
            Some(unsafe { &mut *(address as *mut _) })
        } else {
            None
        }
    }

    fn next_table_create(&mut self, index: usize) -> &'static mut UserPageTable<L::NextLevel> {
        debug_assert!(L::ID > 1);
        if let Some(address) = self.next_table_address(index) {
            return unsafe { &mut *(address as *mut _) };
        } else {
            let frame = KERNEL_MEMORY_MAPPER
                .acquire_physical_page::<Size4K>()
                .unwrap();
            self.entries[index].set(frame, PageFlags::page_table_flags());
            ::core::sync::atomic::fence(::core::sync::atomic::Ordering::SeqCst);
            let t = self.next_table_create(index);
            t.zero();
            ::core::sync::atomic::fence(::core::sync::atomic::Ordering::SeqCst);
            t
        }
    }

    #[allow(mutable_transmutes)]
    fn get_entry(&self, address: Address<V>) -> Option<(usize, &'static mut PageTableEntry)> {
        debug_assert!(L::ID != 0);
        let index = Self::get_index(address);
        if L::ID == 2 && self.entries[index].is_block() {
            return Some((L::ID, unsafe {
                ::core::mem::transmute(&self.entries[index])
            }));
        }
        if L::ID == 1 {
            return Some((L::ID, unsafe {
                ::core::mem::transmute(&self.entries[index])
            }));
        }

        let next = self.next_table(index)?;
        next.get_entry(address)
    }

    fn get_entry_create<S: PageSize>(
        &mut self,
        address: Address<V>,
    ) -> (usize, &'static mut PageTableEntry) {
        debug_assert!(L::ID != 0);
        let index = Self::get_index(address);
        if L::ID == 2 && self.entries[index].present() && self.entries[index].is_block() {
            debug_assert!(S::BYTES != Size4K::BYTES);
            return (L::ID, unsafe {
                ::core::mem::transmute(&mut self.entries[index])
            });
        }
        if S::BYTES == Size4K::BYTES && L::ID == 1 {
            return (L::ID, unsafe {
                ::core::mem::transmute(&mut self.entries[index])
            });
        }
        if S::BYTES == Size2M::BYTES && L::ID == 2 {
            return (L::ID, unsafe {
                ::core::mem::transmute(&mut self.entries[index])
            });
        }

        let next = self.next_table_create(index);

        next.get_entry_create::<S>(address)
    }
}

impl UserPageTable<L4> {
    pub const fn new() -> Self {
        Self {
            entries: unsafe { ::core::mem::transmute([0u64; 512]) },
            phantom: PhantomData,
        }
    }

    #[inline]
    pub fn get() -> &'static mut Self {
        unsafe { Address::<V>::new(0x0000_ffff_ffff_f000).as_mut() }
    }

    pub fn translate(&mut self, a: Address<V>) -> Option<(Address<P>, PageFlags)> {
        let (_level, entry) = self.get_entry(a)?;
        if entry.present() {
            let page_offset = a.as_usize() & 0xfff;
            Some((entry.address() + page_offset, entry.flags()))
        } else {
            None
        }
    }

    pub fn identity_map<S: PageSize>(&mut self, frame: Frame<S>, flags: PageFlags) -> Page<S> {
        self.map(Page::new(frame.start().as_usize().into()), frame, flags)
    }

    pub fn map<S: PageSize>(
        &mut self,
        page: Page<S>,
        frame: Frame<S>,
        flags: PageFlags,
    ) -> Page<S> {
        let (level, entry) = self.get_entry_create::<S>(page.start());

        if cfg!(debug_assertions) {
            if S::BYTES == Size4K::BYTES {
                assert!(level == 1, "{:?} {:?} {}", page, frame, level);
            } else if S::BYTES == Size2M::BYTES {
                assert!(level == 2);
            }
        }
        if S::BYTES != Size4K::BYTES {
            debug_assert!(flags.bits() & 0b10 == 0);
        }
        debug_assert!(!entry.present());
        debug_assert!(entry.address().is_zero());
        let flags = flags | PageFlag::PRESENT;
        entry.set(frame, flags);
        page
    }

    pub fn remap<S: PageSize>(
        &mut self,
        page: Page<S>,
        frame: Frame<S>,
        flags: PageFlags,
    ) -> Page<S> {
        let (level, entry) = self.get_entry_create::<S>(page.start());
        if cfg!(debug_assertions) {
            if S::BYTES == Size4K::BYTES {
                assert!(level == 1, "{:?} {:?} {}", page, frame, level);
            } else if S::BYTES == Size2M::BYTES {
                assert!(level == 2);
            }
        }
        if S::BYTES != Size4K::BYTES {
            debug_assert!(flags.bits() & 0b10 == 0);
        }
        let flags = flags | PageFlag::PRESENT;
        entry.set(frame, flags);
        page
    }

    pub fn update_flags<S: PageSize>(&mut self, page: Page<S>, flags: PageFlags) -> Page<S> {
        let (level, entry) = self.get_entry_create::<S>(page.start());
        if cfg!(debug_assertions) {
            if S::BYTES == Size4K::BYTES {
                assert!(level == 1, "{:?} {}", page, level);
            } else if S::BYTES == Size2M::BYTES {
                assert!(level == 2);
            }
        }
        if S::BYTES != Size4K::BYTES {
            debug_assert!(flags.bits() & 0b10 == 0);
        }
        let flags = flags | PageFlag::PRESENT;
        entry.update_flags(flags);
        page
    }

    pub fn unmap<S: PageSize>(&mut self, page: Page<S>) {
        let (level, entry) = self.get_entry(page.start()).unwrap();
        if cfg!(debug_assertions) {
            if S::BYTES == Size4K::BYTES {
                assert!(level == 1);
            } else if S::BYTES == Size2M::BYTES {
                assert!(level == 2);
            }
        }
        entry.clear();
    }

    pub fn map_temporarily<S: PageSize>(
        &mut self,
        frame: Frame<S>,
        flags: PageFlags,
    ) -> TemporaryKernelPage<S> {
        const MAGIC_PAGE: usize = 0x0000_1234_5600_0000;
        let page = Page::new(MAGIC_PAGE.into());
        self.map(page, frame, flags);
        invalidate_tlb();
        TemporaryKernelPage(page)
    }

    // pub fn inactive_map<S: PageSize>(
    //     &mut self,
    //     page: Page<S>,
    //     frame: Frame<S>,
    //     flags: PageFlags,
    // ) -> Page<S> {
    //     // P4
    //     let table = self;
    //     // P3
    //     let index = KernelPageTable::<L4>::get_index(page.start());
    //     if table[index].is_empty() {
    //         table[index].set(Self::alloc_frame4k(), PageFlags::page_table_flags());
    //     }
    //     let table = table.get_next_table(index).unwrap();
    //     // P2
    //     let index = KernelPageTable::<L3>::get_index(page.start());
    //     if table.entries[index].is_empty() {
    //         table.entries[index].set(Self::alloc_frame4k(), PageFlags::page_table_flags());
    //     }
    //     let table = table.get_next_table(index).unwrap();
    //     if S::BYTES == Size2M::BYTES {
    //         table.entries[KernelPageTable::<L2>::get_index(page.start())].set(frame, flags);
    //     }
    //     // P1
    //     let index = KernelPageTable::<L2>::get_index(page.start());
    //     if table.entries[index].is_empty() {
    //         table.entries[index].set(Self::alloc_frame4k(), PageFlags::page_table_flags());
    //     }
    //     let table = table.get_next_table(index).unwrap();
    //     table.entries[KernelPageTable::<L1>::get_index(page.start())]
    //         .set(frame, flags | PageFlag::SMALL_PAGE);
    //     page
    // }

    // pub fn unmap<S: PageSize>(&mut self, _page: Page<S>) {
    //     unimplemented!()
    // }
}

fn invalidate_tlb() {
    unsafe {
        asm! {"
            tlbi vmalle1is
            DSB SY
            isb
        "}
    }
}

// 4096 / 8
// impl <L: TableLevel> PageTable<L> {
//     /// Fork a (user) page table hierarchy
//     ///
//     /// This will copy all page tables and mark all (non-pagetable) pages as copy-on-write.
//     ///
//     /// Special case for kernel stack pages:
//     /// we simply redirect them to new frames, but not responsible for the copying
//     pub fn fork(&mut self) -> Frame {
//         if L::ID == 0 { unreachable!() }

//         // Alloc a new table
//         let new_table_frame = FRAME_ALLOCATOR.alloc::<Size4K>();
//         {
//             let page = map_kernel_temporarily(new_table_frame, PageFlags::_PAGE_TABLE_FLAGS, None);
//             unsafe { page.zero(); }
//         }

//         // Copy entries & recursively fork children
//         let limit = if L::ID == 4 { 511 } else { 512 };
//         for i in 0..limit {
//             if self.entries[i].present() {
//                 if L::ID != 1 && self.entries[i].flags().contains(PageFlags::SMALL_PAGE) {
//                     // This entry is a page table
//                     let table = self.next_table(i).unwrap();
//                     let flags = self.entries[i].flags();
//                     let frame = table.fork();
//                     let page = map_kernel_temporarily(new_table_frame, PageFlags::_PAGE_TABLE_FLAGS, None);
//                     let new_table = unsafe { page.start().as_ref_mut::<Self>() };
//                     new_table.entries[i].set(frame, flags);
//                 } else {
//                     // This entry points to a page, mark as copy-on-write
//                     // let flags = self.entries[i].flags();
//                     // let address = self.entries[i].address();
//                     let page = map_kernel_temporarily(new_table_frame, PageFlags::_PAGE_TABLE_FLAGS, None);
//                     let new_table = unsafe { page.start().as_ref_mut::<Self>() };

//                     let old_flags = self.entries[i].flags();
//                     if old_flags.contains(PageFlags::NO_WRITE) {
//                         // FIXME: What if child process updates this flag as writeable?
//                         continue; // Skip since it is readonly already
//                     }
//                     let flags = old_flags | PageFlags::COPY_ON_WRITE | PageFlags::NO_WRITE;
//                     let addr = self.entries[i].address();
//                     self.entries[i].update_flags(flags);
//                     if flags.contains(PageFlags::SMALL_PAGE) {
//                         new_table.entries[i].set::<Size4K>(Frame::new(addr), flags);
//                     } else {
//                         new_table.entries[i].set::<Size2M>(Frame::new(addr), flags);
//                     }
//                 }
//             }
//         }

//         if L::ID == 4 {
//             // Recursively reference P4 itself
//             let page = map_kernel_temporarily(new_table_frame, PageFlags::_PAGE_TABLE_FLAGS, None);
//             let new_table = unsafe { page.start().as_ref_mut::<PageTable<L4>>() };
//             new_table.entries[511].set(new_table_frame, PageFlags::_PAGE_TABLE_FLAGS);
//         }

//         new_table_frame
//     }

// }

// impl PageTable<L4> {
//     pub fn fix_copy_on_write(&mut self, a: Address, is_large_page: bool) {
//         let p3 = self.next_table(PageTable::<L4>::get_index(a)).unwrap();
//         let p2 = p3.next_table(PageTable::<L3>::get_index(a)).unwrap();
//         if is_large_page {
//             unimplemented!();
//         } else {
//             let p1 = p2.next_table(PageTable::<L2>::get_index(a)).unwrap();
//             let p1_index = PageTable::<L1>::get_index(a);
//             debug_assert!(p1.entries[p1_index].flags().contains(PageFlags::COPY_ON_WRITE));
//             let old_page = Page::<Size4K>::of(a);
//             let new_frame = FRAME_ALLOCATOR.alloc::<Size4K>();
//             {
//                 let new_page = map_kernel_temporarily(new_frame, PageFlags::_USER_STACK_FLAGS, None);
//                 let mut offset = 0;
//                 while offset < Size4K::SIZE {
//                     let old_word = old_page.start() + offset;
//                     let new_word = new_page.start() + offset;
//                     unsafe {
//                         new_word.store::<usize>(old_word.load());
//                     }
//                     offset += Address::<V>::SIZE;
//                 }
//             }
//             let new_flags = p1.entries[p1_index].flags() - PageFlags::COPY_ON_WRITE - PageFlags::NO_WRITE;
//             p1.entries[p1_index].set(new_frame, new_flags);
//         }
//     }
// }

// use core::ops::*;

pub struct TemporaryKernelPage<S: PageSize>(Page<S>);

impl<S: PageSize> Deref for TemporaryKernelPage<S> {
    type Target = Page<S>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<S: PageSize> Drop for TemporaryKernelPage<S> {
    fn drop(&mut self) {
        UserPageTable::<L4>::get().unmap(self.0);
        invalidate_tlb();
    }
}

// pub fn map_kernel_temporarily<S: PageSize>(frame: Frame<S>, flags: PageFlags, p: Option<usize>) -> TemporaryKernelPage<S> {
//     log!("map_kernel_temporarily 1 TTBR {:#x} {:#x}", TTBR0_EL1.get(), TTBR1_EL1.get());
//     const MAGIC_PAGE: usize = 0x0000_1234_5600_0000;
//     log!("map_kernel_temporarily 2");
//     let page = Page::new(p.unwrap_or(MAGIC_PAGE).into());
//     log!("map_kernel_temporarily 3");
//     PageTable::<L4>::get(false).map(page, frame, flags);
//     log!("map_kernel_temporarily 4");
//     // map_kernel(page, frame, flags);
//     super::paging::invalidate_tlb();
//     log!("map_kernel_temporarily 5");
//     TemporaryKernelPage(page)
// }
