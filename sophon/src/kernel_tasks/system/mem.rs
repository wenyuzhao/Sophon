use crate::task::*;

pub fn map_physical_memory(_m: &Message) {
    unreachable!()
    // let (frame, page) = m.get_data::<(Frame, Page)>();
    // debug!(K: "{:?} -> {:?}", frame, page);
    // let flags = PageFlags::PAGE_4K | PageFlags::PRESENT | PageFlags::ACCESSED;
    // <K::Arch as AbstractArch>::MemoryManager::map_user(m.sender, *page, *frame, flags);

    // let reply_parent = Message::new(m.receiver, m.sender, 0)
    //     .with_data(page.start());
    // reply_parent.send();
}
