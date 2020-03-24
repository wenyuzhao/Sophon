use crate::task::*;
use crate::arch::*;
use crate::utils::address::*;
use crate::utils::page::*;
use crate::memory::PageFlags;

pub fn map_physical_memory(m: &Message) {
    let (frame, page) = m.get_data::<(Frame, Page)>();
    println!("{:?} -> {:?}", frame, page);
    let flags = PageFlags::PAGE_4K | PageFlags::PRESENT | PageFlags::ACCESSED;
    Target::MemoryManager::map(*page, *frame, flags);

    let reply_parent = Message::new(m.receiver, m.sender, 0)
        .with_data(page.start());
    reply_parent.send();
}