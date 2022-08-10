use core::sync::atomic::Ordering;

use alloc::boxed::Box;
use bitflags::bitflags;
use kernel_module::SERVICE;
use memory::address::{Address, P, V};

#[bitflags(u16)]
pub enum VirtQueueDescFlags {
    None = 0,
    Next = 1,
    Write = 2,
    Indirect = 4,
}

#[repr(C, align(16))]
pub struct VirtQueueDesc {
    pub addr: Address<P>,
    pub len: u32,
    pub flags: VirtQueueDescFlags,
    pub next: u16,
}

#[repr(C, align(2))]
pub struct VirtQueueAvail {
    pub flags: u16,
    pub index: u16,
    pub ring: [u16; 128],
    pub used_event: u16,
}

#[repr(C, align(4))]
pub struct VirtQueueUsed {
    pub flags: u16,
    pub index: u16,
    pub ring: [VirtQueueUsedElem; 128],
    pub avail_event: u16,
}

#[repr(C)]
pub struct VirtQueueUsedElem {
    pub id: u32,
    pub len: u32,
}

#[repr(C)]
pub struct VirtQueue {
    pub desc: &'static mut [VirtQueueDesc; 128],
    pub avail: &'static mut VirtQueueAvail,
    pub used: &'static mut VirtQueueUsed,
    desc_base: Address<P>,
    avail_base: Address<P>,
    used_base: Address<P>,
    free_desc: u16,
    pub desc_virtual_ptrs: [Address<V>; 128],
}

impl VirtQueue {
    pub fn new() -> &'static mut Self {
        let v_addr = SERVICE.alloc_pages(1).unwrap().start.start();
        let desc_base = SERVICE.translate(v_addr).unwrap();
        let desc = unsafe { v_addr.as_mut::<[VirtQueueDesc; 128]>() };
        let v_addr = SERVICE.alloc_pages(1).unwrap().start.start();
        let avail_base = SERVICE.translate(v_addr).unwrap();
        let avail = unsafe { v_addr.as_mut::<VirtQueueAvail>() };
        let v_addr = SERVICE.alloc_pages(1).unwrap().start.start();
        let used_base = SERVICE.translate(v_addr).unwrap();
        let used = unsafe { v_addr.as_mut::<VirtQueueUsed>() };
        for i in 0..128 {
            desc[i].next = (i + 1) as _;
        }
        Box::leak(Box::new(Self {
            desc,
            avail,
            used,
            desc_base,
            avail_base,
            used_base,
            free_desc: 0,
            desc_virtual_ptrs: [Address::ZERO; 128],
        }))
    }

    pub fn size(&self) -> usize {
        128
    }

    pub fn desc_start(&self) -> Address<P> {
        self.desc_base
    }

    pub fn avail_start(&self) -> Address<P> {
        self.avail_base
    }

    pub fn used_start(&self) -> Address<P> {
        self.used_base
    }

    fn find_desc(&mut self) -> u16 {
        let desc = self.free_desc;
        assert!(desc <= self.size() as u16);
        self.free_desc = self.desc[desc as usize].next;
        desc
    }

    pub fn push<T: Sized>(&mut self, data: &T, flags: VirtQueueDescFlags) -> usize {
        let desc = self.find_desc() as usize;
        println!("desc: {}", desc);
        self.desc[desc].len = core::mem::size_of::<T>() as _;
        self.desc[desc].addr = SERVICE.translate(Address::from(&data)).unwrap();
        self.desc[desc].flags = flags;
        core::sync::atomic::fence(Ordering::SeqCst);
        desc
    }

    pub fn update_avail(&mut self, desc: u16) {
        self.avail.ring[self.avail.index as usize] = desc;
        core::sync::atomic::fence(Ordering::SeqCst);
        self.avail.index += 1;
        core::sync::atomic::fence(Ordering::SeqCst);
    }
}
