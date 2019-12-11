use cortex_a::regs::*;
use core::marker::PhantomData;
use super::frame_allocator;
use super::address::*;
use super::page::*;
use super::heap_constants::*;


bitflags! {
    pub struct PageFlags: usize {
        const PRESENT     = 0b01;      // map a 4k page
        const SMALL_PAGE  = 0b10;      // map a 4k page
        const USER        = 1 << 6;    // enable EL0 Access
        const NO_WRITE    = 1 << 7;    // readonly
        const ACCESSED    = 1 << 10;   // accessed
        const NO_EXEC     = 1 << 54;   // no execute
        const INNER_SHARE = 0b10 << 8; // outter shareable
        const OUTER_SHARE = 0b11 << 8; // inner shareable
        const COPY_ON_WRITE = 1 << 53;
        const NORMAL_MEMORY = 0b001 << 2;
        const DEVICE_MEMORY = 0b000 << 2;

        // Commonly used flags
        const _DEVICE_MEMORY_FLAGS_4K = Self::PRESENT.bits | Self::SMALL_PAGE.bits | Self::OUTER_SHARE.bits | Self::ACCESSED.bits;
        const _DEVICE_MEMORY_FLAGS_2M = Self::PRESENT.bits | Self::OUTER_SHARE.bits | Self::ACCESSED.bits;
        const _PAGE_TABLE_FLAGS = Self::NORMAL_MEMORY.bits | Self::NO_EXEC.bits | Self::PRESENT.bits | Self::SMALL_PAGE.bits | Self::OUTER_SHARE.bits | Self::ACCESSED.bits;
        const _KERNEL_STACK_FLAGS = Self::NORMAL_MEMORY.bits | Self::NO_EXEC.bits | Self::PRESENT.bits | Self::SMALL_PAGE.bits | Self::OUTER_SHARE.bits | Self::ACCESSED.bits;
        const _KERNEL_STACK_GUARD_FLAGS = Self::NORMAL_MEMORY.bits | !Self::ACCESSED.bits & (Self::NO_WRITE.bits | Self::_KERNEL_STACK_FLAGS.bits);
        const _KERNEL_CODE_FLAGS_2M = Self::NORMAL_MEMORY.bits | Self::PRESENT.bits | Self::OUTER_SHARE.bits | Self::ACCESSED.bits;
        const _KERNEL_CODE_FLAGS_4K = Self::NORMAL_MEMORY.bits | Self::_KERNEL_CODE_FLAGS_2M.bits | Self::SMALL_PAGE.bits;
        const _KERNEL_DATA_FLAGS_2M = Self::NORMAL_MEMORY.bits | Self::NO_EXEC.bits | Self::PRESENT.bits | Self::OUTER_SHARE.bits | Self::ACCESSED.bits;
        const _KERNEL_DATA_FLAGS_4K = Self::NORMAL_MEMORY.bits | Self::_KERNEL_DATA_FLAGS_2M.bits | Self::SMALL_PAGE.bits;
        const _USER_STACK_FLAGS = Self::NORMAL_MEMORY.bits | Self::_KERNEL_STACK_FLAGS.bits | Self::USER.bits;
        // FIXME: Should we mark code pages as `NO_WRITE`?
        const _USER_CODE_FLAGS = Self::NORMAL_MEMORY.bits | Self::_KERNEL_CODE_FLAGS_4K.bits | Self::USER.bits;
    }
}

#[repr(C)]
#[derive(Clone)]
pub struct PageTableEntry(usize);

impl PageTableEntry {
    const ADDRESS_MASK: usize = 0x0000_ffff_ffff_f000;
    const FLAGS_MASK: usize = !Self::ADDRESS_MASK;
    
    pub fn clear(&mut self) {
        self.0 = 0;
    }
    pub fn present(&self) -> bool {
        self.flags().contains(PageFlags::PRESENT)
    }
    pub fn is_block(&self) -> bool {
        !self.flags().contains(PageFlags::SMALL_PAGE)
    }
    pub fn address(&self) -> Address<P> {
        (self.0 & Self::ADDRESS_MASK).into()
    }
    pub fn flags(&self) -> PageFlags {
        let v = self.0 & Self::FLAGS_MASK;
        PageFlags::from_bits_truncate(v)
    }
    pub fn update_flags(&mut self, new_flags: PageFlags) {
        self.0 = self.address().as_usize() | new_flags.bits();
    }
    pub fn set<S: PageSize>(&mut self, frame: Frame<S>, flags: PageFlags) {
        if S::LOG_SIZE == Size2M::LOG_SIZE {
            debug_assert!(flags.bits() & 0b10 == 0);
        } else {
            debug_assert!(flags.bits() & 0b10 == 0b10);
        }
        let mut a = frame.start().as_usize();
        a &= !(0xffff_0000_0000_0000);
        self.0 = a | flags.bits();
    }
}

pub trait TableLevel {
    const ID: usize;
    const SHIFT: usize;
    type NextLevel: TableLevel;
}

pub struct L4;

impl TableLevel for L4 {
    const ID: usize = 4;
    const SHIFT: usize = 12 + 9 * 3;
    type NextLevel = L3;
}

pub struct L3;

impl TableLevel for L3 {
    const ID: usize = 3;
    const SHIFT: usize = 12 + 9 * 2;
    type NextLevel = L2;
}

pub struct L2;

impl TableLevel for L2 {
    const ID: usize = 2;
    const SHIFT: usize = 12 + 9 * 1;
    type NextLevel = L1;
}

pub struct L1;

impl TableLevel for L1 {
    const ID: usize = 1;
    const SHIFT: usize = 12 + 9 * 0;
    type NextLevel = !;
}

impl TableLevel for ! {
    const ID: usize = 0;
    const SHIFT: usize = 0;
    type NextLevel = !;
}

#[repr(C, align(4096))]
pub struct PageTable<L: TableLevel + 'static> {
    pub entries: [PageTableEntry; 512],
    phantom: PhantomData<L>,
}

impl <L: TableLevel> PageTable<L> {
    const MASK: usize = 0b111111111 << L::SHIFT;
    fn zero(&mut self) {
        for i in 0..512 {
            self.entries[i] = PageTableEntry(0);
        }
    }

    #[inline]
    fn get_index(a: Address<V>) -> usize {
        (a.as_usize() >> L::SHIFT) & 0b111111111
    }

    fn next_table_address(&self, index: usize) -> Option<usize> {
        // crate::debug_boot::log("next_table_address 0");
        debug_assert!(L::ID > 1);
        // crate::debug_boot::log("next_table_address 1");
        debug_assert!(index < 512);
        // crate::debug_boot::log("next_table_address 1.1");
        let x = self.entries[index].present();
        // crate::debug_boot::log("next_table_address 1.2");
        if self.entries[index].present() && !self.entries[index].is_block() {
            if crate::mm::booted() { println!("next_table_address {} {:x}", index, self.entries[index].0); }
            // crate::debug_boot::log("next_table_address 2");
            let table_address = self as *const _ as usize;
            // crate::debug_boot::log("next_table_address 3");
            let mut a = (table_address << 9) | (index << 12);
            // crate::debug_boot::log("next_table_address 4");
            if self as *const _ as usize & (0xffff << 48) == 0 {
                // crate::debug_boot::log("next_table_address 5");
                a &= 0x0000_ffff_ffff_ffff;
            }
            // crate::debug_boot::log("next_table_address 6");
            Some(a)
        } else {
            // crate::debug_boot::log("next_table_address -> None");
            None
        }
    }

    fn next_table(&self, index: usize) ->  Option<&'static mut PageTable<L::NextLevel>> {
        // crate::debug_boot::log("next_table 0");
        debug_assert!(L::ID > 1);
        // crate::debug_boot::log("next_table 1");
        if let Some(address) = self.next_table_address(index) {
            // crate::debug_boot::log("next_table 2");
            Some(unsafe { &mut *(address as *mut _) })
        } else {
            // crate::debug_boot::log("next_table 3");
            None
        }
    }

    fn next_table_create(&mut self, index: usize) -> &'static mut PageTable<L::NextLevel> {
        debug_assert!(L::ID > 1);
        if let Some(address) = self.next_table_address(index) {
            return unsafe { &mut *(address as *mut _) }
        } else {
            if crate::mm::booted() { println!("ntc 2"); }
            let frame = frame_allocator::alloc::<Size4K>().expect("no framxes available");
            if crate::mm::booted() { println!("ntc 3"); }
            // boot_log!("Alloc frame {:?}", frame);
            self.entries[index].set(frame, PageFlags::_PAGE_TABLE_FLAGS);
            if crate::mm::booted() { println!("ntc 4"); }
            // boot_log!("self.entries[{:?}]", index);
            let t = self.next_table_create(index);
            if crate::mm::booted() { println!("ntc 5"); }
            // boot_log!("next_table_create end {:?}", t as *const _);
            t.zero();
            if crate::mm::booted() { println!("ntc 6"); }
            // boot_log!("zeroed");
            t
        }
    }

    #[allow(mutable_transmutes)]
    fn get_entry(&self, address: Address<V>) -> Option<(usize, &'static mut PageTableEntry)> {
        debug_assert!(L::ID != 0);
        // crate::debug_boot::log("get_entry 0");
        let index = Self::get_index(address);
        // crate::debug_boot::log("get_entry 1");
        if L::ID == 2 && self.entries[index].is_block() {
            // crate::debug_boot::log("get_entry 2");
            return Some((L::ID, unsafe { ::core::mem::transmute(&self.entries[index]) }));
        }
        if L::ID == 1 {
            // crate::debug_boot::log("get_entry 3");
            return Some((L::ID, unsafe { ::core::mem::transmute(&self.entries[index]) }));
        }
        
        // crate::debug_boot::log("get_entry 4");
        let next = self.next_table(index)?;
        // crate::debug_boot::log("get_entry 5");
        next.get_entry(address)
    }

    fn get_entry_create<S: PageSize>(&mut self, address: Address<V>) -> (usize, &'static mut PageTableEntry) {
        // crate::debug_boot::log("get_entry_create 0");
        // if crate::mm::booted() { println!("gec 0"); }
        debug_assert!(L::ID != 0);
        // crate::debug_boot::log("get_entry_create 1");
        // if crate::mm::booted() { println!("gec 1"); }
        let index = Self::get_index(address);
        // crate::debug_boot::log("get_entry_create 2");
        // if crate::mm::booted() { println!("gec 2"); }
        if L::ID == 2 && self.entries[index].present() && self.entries[index].is_block() {
            // crate::debug_boot::log("get_entry_create 3");
            // if crate::mm::booted() { println!("gec 3"); }
            debug_assert!(S::LOG_SIZE != Size4K::LOG_SIZE);
            return (L::ID, unsafe { ::core::mem::transmute(&mut self.entries[index]) });
        }
        if S::LOG_SIZE == Size4K::LOG_SIZE && L::ID == 1 {
            // crate::debug_boot::log("get_entry_create 4");
            // if crate::mm::booted() { println!("gec 4"); }
            return (L::ID, unsafe { ::core::mem::transmute(&mut self.entries[index]) });
        }
        if S::LOG_SIZE == Size2M::LOG_SIZE && L::ID == 2 {
            // crate::debug_boot::log("get_entry_create 5");
            // if crate::mm::booted() { println!("gec 5"); }
            return (L::ID, unsafe { ::core::mem::transmute(&mut self.entries[index]) });
        }
        // crate::debug_boot::log("get_entry_create 6");
        if crate::mm::booted() { println!("gec 6"); }
        let next = self.next_table_create(index);
        // crate::debug_boot::log("get_entry_create 7");
        if crate::mm::booted() { println!("gec 7 {:?}", next as *mut _); }
        next.get_entry_create::<S>(address)
    }
}

impl PageTable<L4> {
    pub const fn new() -> Self {
        Self {
            entries: unsafe { ::core::mem::transmute([0u64; 512]) },
            phantom: PhantomData,
        }
    }

    #[inline]
    pub fn get(high: bool) -> &'static mut Self {
        if high {
            unsafe { Address::<V>::new(0xffff_ffff_ffff_f000).as_ref_mut() }
        } else {
            unsafe { Address::<V>::new(0x0000_ffff_ffff_f000).as_ref_mut() }
        }
    }

    pub fn translate(&mut self, a: Address<V>) -> Option<(Address<P>, PageFlags)> {
        // crate::debug_boot::log("get_entry start");
        let (level, entry) = self.get_entry(a)?;
        // crate::debug_boot::log("get_entry finished");
        if entry.present() {
            let page_offset = a.as_usize() & 0xfff;
            Some((entry.address() + page_offset, entry.flags()))
        } else {
            None
        }
    }

    pub fn identity_map<S: PageSize>(&mut self, frame: Frame<S>, flags: PageFlags) -> Page<S> {
        let u = frame.start().as_usize();
        self.map(Page::new(frame.start().as_usize().into()), frame, flags)
    }

    pub fn map<S: PageSize>(&mut self, page: Page<S>, frame: Frame<S>, flags: PageFlags) -> Page<S> {
        if crate::mm::booted() { println!("map 0"); }
        let (level, entry) = self.get_entry_create::<S>(page.start());
        if crate::mm::booted() { println!("map 1"); }
        if cfg!(debug_assertions) {
            if S::LOG_SIZE == Size4K::LOG_SIZE {
                assert!(level == 1, "{:?} {:?} {}", page, frame, level);
            } else if S::LOG_SIZE == Size2M::LOG_SIZE {
                assert!(level == 2);
            }
        }
        // if crate::mm::booted() { println!("map 2"); }
        if S::LOG_SIZE != Size4K::LOG_SIZE {
            debug_assert!(flags.bits() & 0b10 == 0);
        }
        // if crate::mm::booted() { println!("map 3"); }
        let flags = flags | PageFlags::PRESENT;
        
        // if crate::mm::booted() { println!("map 4"); }
        entry.set(frame, flags);
        
        // if crate::mm::booted() { println!("map 5"); }
        page
    }

    pub fn remap<S: PageSize>(&mut self, page: Page<S>, frame: Frame<S>, flags: PageFlags) -> Page<S> {
        let (level, entry) = self.get_entry_create::<S>(page.start());
        if cfg!(debug_assertions) {
            if S::LOG_SIZE == Size4K::LOG_SIZE {
                assert!(level == 1, "{:?} {:?} {}", page, frame, level);
            } else if S::LOG_SIZE == Size2M::LOG_SIZE {
                assert!(level == 2);
            }
        }
        if S::LOG_SIZE != Size4K::LOG_SIZE {
            debug_assert!(flags.bits() & 0b10 == 0);
        }
        let flags = flags | PageFlags::PRESENT;
        entry.set(frame, flags);
        page
    }

    pub fn update_flags<S: PageSize>(&mut self, page: Page<S>, flags: PageFlags) -> Page<S> {
        let (level, entry) = self.get_entry_create::<S>(page.start());
        if cfg!(debug_assertions) {
            if S::LOG_SIZE == Size4K::LOG_SIZE {
                assert!(level == 1, "{:?} {}", page, level);
            } else if S::LOG_SIZE == Size2M::LOG_SIZE {
                assert!(level == 2);
            }
        }
        if S::LOG_SIZE != Size4K::LOG_SIZE {
            debug_assert!(flags.bits() & 0b10 == 0);
        }
        let flags = flags | PageFlags::PRESENT;
        entry.update_flags(flags);
        page
    }

    pub fn unmap<S: PageSize>(&mut self, page: Page<S>) {
        let (level, entry) = self.get_entry(page.start()).unwrap();
        if cfg!(debug_assertions) {
            if S::LOG_SIZE == Size4K::LOG_SIZE {
                assert!(level == 1);
            } else if S::LOG_SIZE == Size2M::LOG_SIZE {
                assert!(level == 2);
            }
        }
        entry.clear();
    }

    pub fn with_temporary_low_table<R>(new_p4_frame: Frame, f: impl Fn(&'static mut PageTable<L4>) -> R) -> R {
        let old_p4_frame = TTBR0_EL1.get();
        TTBR0_EL1.set(new_p4_frame.start().as_usize() as u64);
        crate::mm::paging::invalidate_tlb();
        let r = f(Self::get(false));
        TTBR0_EL1.set(new_p4_frame.start().as_usize() as u64);
        crate::mm::paging::invalidate_tlb();
        r
    }
}

// 4096 / 8
impl <L: TableLevel> PageTable<L> {
    /// Fork a (user) page table hierarchy
    /// 
    /// This will copy all page tables and mark all (non-pagetable) pages as copy-on-write.
    /// 
    /// Special case for kernel stack pages:
    /// we simply redirect them to new frames, but not responsible for the copying
    pub fn fork(&mut self) -> Frame {
        if L::ID == 0 { unreachable!() }

        // Alloc a new table
        let new_table_frame = frame_allocator::alloc::<Size4K>().unwrap();
        {
            let page = crate::mm::map_kernel_temporarily(new_table_frame, PageFlags::_PAGE_TABLE_FLAGS, None);
            unsafe { page.zero(); }
        }
        
        // Copy entries & recursively fork children
        let limit = if L::ID == 4 { 511 } else { 512 };
        for i in 0..limit {
            if self.entries[i].present() {
                if L::ID != 1 && self.entries[i].flags().contains(PageFlags::SMALL_PAGE) {
                    // This entry is a page table
                    let table = self.next_table(i).unwrap();
                    let flags = self.entries[i].flags();
                    let frame = table.fork();
                    let page = crate::mm::map_kernel_temporarily(new_table_frame, PageFlags::_PAGE_TABLE_FLAGS, None);
                    let new_table = unsafe { page.start().as_ref_mut::<Self>() };
                    new_table.entries[i].set(frame, flags);
                } else {
                    // This entry points to a page, mark as copy-on-write
                    let flags = self.entries[i].flags();
                    let address = self.entries[i].address();
                    let page = crate::mm::map_kernel_temporarily(new_table_frame, PageFlags::_PAGE_TABLE_FLAGS, None);
                    let new_table = unsafe { page.start().as_ref_mut::<Self>() };

                    let old_flags = self.entries[i].flags();
                    if old_flags.contains(PageFlags::NO_WRITE) {
                        // FIXME: What if child process updates this flag as writeable?
                        continue; // Skip since it is readonly already
                    }
                    let flags = old_flags | PageFlags::COPY_ON_WRITE | PageFlags::NO_WRITE;
                    let addr = self.entries[i].address();
                    self.entries[i].update_flags(flags);
                    if flags.contains(PageFlags::SMALL_PAGE) {
                        new_table.entries[i].set::<Size4K>(Frame::new(addr), flags);
                    } else {
                        new_table.entries[i].set::<Size2M>(Frame::new(addr), flags);
                    }
                }
            }
        }

        if L::ID == 4 {
            // Recursively reference P4 itself
            let page = crate::mm::map_kernel_temporarily(new_table_frame, PageFlags::_PAGE_TABLE_FLAGS, None);
            let new_table = unsafe { page.start().as_ref_mut::<PageTable<L4>>() };
            new_table.entries[511].set(new_table_frame, PageFlags::_PAGE_TABLE_FLAGS);
        }

        new_table_frame
    }

}

impl PageTable<L4> {
    pub fn fix_copy_on_write(&mut self, a: Address, is_large_page: bool) {
        let p3 = self.next_table(PageTable::<L4>::get_index(a)).unwrap();
        let p2 = p3.next_table(PageTable::<L3>::get_index(a)).unwrap();
        if is_large_page {
            unimplemented!();
        } else {
            let p1 = p2.next_table(PageTable::<L2>::get_index(a)).unwrap();
            let p1_index = PageTable::<L1>::get_index(a);
            debug_assert!(p1.entries[p1_index].flags().contains(PageFlags::COPY_ON_WRITE));
            let old_page = Page::<Size4K>::of(a);
            let new_frame = frame_allocator::alloc::<Size4K>().unwrap();
            {
                let new_page = crate::mm::map_kernel_temporarily(new_frame, PageFlags::_USER_STACK_FLAGS, None);
                let mut offset = 0;
                while offset < Size4K::SIZE {
                    let old_word = old_page.start() + offset;
                    let new_word = new_page.start() + offset;
                    unsafe {
                        new_word.store::<usize>(old_word.load());
                    }
                    offset += Address::<V>::SIZE;
                }
            }
            let new_flags = p1.entries[p1_index].flags() - PageFlags::COPY_ON_WRITE - PageFlags::NO_WRITE;
            p1.entries[p1_index].set(new_frame, new_flags);
        }
    }
}