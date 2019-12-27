use crate::task::*;
// use crate::arch::*;
// use crate::utils::address::*;
// use crate::utils::page::*;


pub fn physical_memory(_m: &Message) {
    unimplemented!()
    // const PHYSICAL_MEMORY_READ: u64 = 0;
    // const PHYSICAL_MEMORY_WRITE: u64 = 1;
    // let (op, src, dst, bytes) = m.get_data::<(u64, Address<V>, Address<P>, u64)>();
    // assert!(bytes < Size4K::SIZE)
    // // 1. Map physical memory
    // let dst_page = Frame::<Size4K>::of(dst);
    // let dst_offset = Frame::of(dst);
    // let Target::MemoryManager::map_temporarily(Page::new(0x200000.into()), );
    // // Copy or write
    // if *op == PHYSICAL_MEMORY_READ {
    //     // 1. Map this physical address
    //     // 2. Copy
    //     ipc::reply(m, 0isize);
    // } else if *op == PHYSICAL_MEMORY_WRITE {
    //     ipc::reply(m, -1isize);
    // } else {
    //     ipc::reply(m, -1isize);
    // }
}