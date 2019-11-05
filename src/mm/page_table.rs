use cortex_a::regs::*;
use core::marker::PhantomData;
use super::frame_allocator;
use super::address::*;
use super::page::*;


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
    }
}

#[repr(C)]
pub struct PageTableEntry(usize);

impl PageTableEntry {
    pub fn present(&self) -> bool {
        self.0 & 0b1 != 0
    }
    pub fn is_block(&self) -> bool {
        self.0 & 0b10 == 0
    }
    pub fn address(&self) -> Address<P> {
        (self.0 & (0b111111111 << 39)).into()
    }
    pub fn flags(&self) -> PageFlags {
        let v = self.0 & !(0b111111111 << 39);
        PageFlags::from_bits_truncate(v)
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
        debug_assert!(L::ID > 1);
        if self.entries[index].present() && !self.entries[index].is_block() {
            let table_address = self as *const _ as usize;
            Some((table_address << 9) | (index << 12))
        } else {
            None
        }
    }

    fn next_table(&self, index: usize) ->  Option<&'static mut PageTable<L::NextLevel>> {
        debug_assert!(L::ID > 1);
        if let Some(address) = self.next_table_address(index) {
            Some(unsafe { &mut *(address as *mut _) })
        } else {
            None
        }
    }

    fn next_table_create(&mut self, index: usize) -> &'static mut PageTable<L::NextLevel> {
        debug_assert!(L::ID > 1);
        if let Some(address) = self.next_table_address(index) {
            return unsafe { &mut *(address as *mut _) }
        } else {
            let frame = frame_allocator::alloc::<Size4K>().expect("no framxes available");
            self.entries[index].set(frame, PageFlags::PRESENT | PageFlags::SMALL_PAGE | PageFlags::OUTER_SHARE | PageFlags::ACCESSED);
            let t = self.next_table_create(index);
            t.zero();
            t
        }
    }

    #[allow(mutable_transmutes)]
    fn get_entry(&self, address: Address<V>) -> Option<(usize, &'static mut PageTableEntry)> {
        debug_assert!(L::ID != 0);
        let index = Self::get_index(address);
        if L::ID == 2 && self.entries[index].is_block() {
            return Some((L::ID, unsafe { ::core::mem::transmute(&self.entries[index]) }));
        }
        if L::ID == 1 {
            return Some((L::ID, unsafe { ::core::mem::transmute(&self.entries[index]) }));
        }
        let next = self.next_table(index)?;
        next.get_entry(address)
    }

    fn get_entry_create<S: PageSize>(&mut self, address: Address<V>) -> (usize, &'static mut PageTableEntry) {
        debug_assert!(L::ID != 0);
        let index = Self::get_index(address);
        if L::ID == 2 && self.entries[index].present() && self.entries[index].is_block() {
            return (L::ID, unsafe { ::core::mem::transmute(&mut self.entries[index]) });
        }
        
        if S::LOG_SIZE == Size4K::LOG_SIZE && L::ID == 1 {
            return (L::ID, unsafe { ::core::mem::transmute(&mut self.entries[index]) });
        }
        if S::LOG_SIZE == Size2M::LOG_SIZE && L::ID == 2 {
            return (L::ID, unsafe { ::core::mem::transmute(&mut self.entries[index]) });
        }
        let next = self.next_table_create(index);
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

    pub fn get() -> &'static mut Self {
        unsafe { Address::<V>::new(0xffff_ffff_ffff_f000).as_ref_mut() }
    }


    pub fn translate(&mut self, a: Address<V>) -> Option<(Address<P>, PageFlags)> {
        let (level, entry) = self.get_entry(a)?;
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

    pub fn map<S: PageSize>(&mut self, page: Page<S>, frame: Frame<S>, flags: PageFlags) -> Page<S> {
        let (level, entry) = self.get_entry_create::<S>(page.start());
        if cfg!(debug_assertions) {
            if S::LOG_SIZE == Size4K::LOG_SIZE {
                assert!(level == 1);
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
}
