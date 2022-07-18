#![no_std]
#![feature(step_trait)]
#![feature(format_args_nl)]

#[allow(unused_imports)]
#[macro_use]
extern crate log;

use core::ops::Range;

use memory::address::Address;
use memory::page::{Page, PageSize, Size4K};
use xmas_elf::{
    dynamic,
    program::{ProgramHeader, SegmentData, Type},
    sections::Rela,
    ElfFile,
};

pub struct ELFLoader<'a, 'b> {
    data: &'a [u8],
    elf: ElfFile<'a>,
    base: Address,
    map_pages: &'b mut dyn FnMut(usize) -> Range<Page>,
}

impl<'a, 'b> ELFLoader<'a, 'b> {
    fn new(data: &'a [u8], map_pages: &'b mut dyn FnMut(usize) -> Range<Page>) -> Self {
        ELFLoader {
            data: data,
            elf: ElfFile::new(data).unwrap(),
            base: Address::ZERO,
            map_pages,
        }
    }

    fn map_memory(&mut self) -> Result<(), &'static str> {
        // Calculate load range
        let (mut load_start, mut load_end) = (None, None);
        let mut update_load_range = |start: Address, end: Address| match (load_start, load_end) {
            (None, None) => (load_start, load_end) = (Some(start), Some(end)),
            (Some(s), Some(e)) => {
                if start < s {
                    load_start = Some(start)
                }
                if end > e {
                    load_end = Some(end)
                }
            }
            _ => unreachable!(),
        };
        for p in self
            .elf
            .program_iter()
            .filter(|ph| ph.get_type() == Ok(Type::Load))
        {
            // log!("{:?}", p);
            let start: Address = (p.virtual_addr() as usize).into();
            let end = start + (p.mem_size() as usize);
            update_load_range(start, end);
        }
        let vaddr_start = Page::<Size4K>::align(load_start.unwrap()).as_usize();
        let vaddr_end = load_end.unwrap().align_up(Size4K::BYTES).as_usize();
        // log!("vaddr: {:?} .. {:?}", vaddr_start, vaddr_end);
        // Map pages
        let pages = (vaddr_end - vaddr_start) >> Page::<Size4K>::LOG_BYTES;
        let pages = (self.map_pages)(pages);
        self.base = pages.start.start();
        Ok(())
    }

    fn load_segment(&self, ph: ProgramHeader) -> Result<(), &'static str> {
        // Copy data
        let start: Address = self.base + ph.virtual_addr() as usize;
        let bytes = ph.file_size() as usize;
        let offset = ph.offset() as usize;
        // log!("copy_nonoverlapping: dst={:?}", start..(start + bytes));
        unsafe {
            core::ptr::copy_nonoverlapping::<u8>(&self.data[offset], start.as_mut_ptr(), bytes);
        }
        // Zero data
        if ph.mem_size() > ph.file_size() {
            let zero_start = start + ph.file_size() as usize;
            let zero_end = start + ph.mem_size() as usize;
            unsafe {
                core::ptr::write_bytes::<u8>(zero_start.as_mut_ptr(), 0, zero_end - zero_start);
            }
        }
        // Flush cache
        memory::cache::flush_cache(start..start + bytes);
        Ok(())
    }

    fn apply_relocation(&self, ph: ProgramHeader) -> Result<(), &'static str> {
        let data = match ph.get_data(&self.elf)? {
            SegmentData::Dynamic64(data) => data,
            _ => unreachable!(),
        };
        let rela = data.iter().find_map(|x| {
            if x.get_tag().ok()? == dynamic::Tag::Rela {
                Some(x.get_ptr().ok()? as usize)
            } else {
                None
            }
        });
        let rela_offset = match rela {
            Some(x) => x,
            _ => return Ok(()),
        };
        let rela_size = data
            .iter()
            .find_map(|x| {
                if x.get_tag().ok()? == dynamic::Tag::RelaSize {
                    Some(x.get_val().ok()? as usize)
                } else {
                    None
                }
            })
            .ok_or("relasize not found")?;
        let rela_ent = data
            .iter()
            .find_map(|x| {
                if x.get_tag().ok()? == dynamic::Tag::RelaEnt {
                    Some(x.get_val().ok()? as usize)
                } else {
                    None
                }
            })
            .ok_or("relaent not found")?;
        let relas = unsafe {
            core::slice::from_raw_parts(
                &self.data[rela_offset] as *const u8 as *const Rela<u64>,
                rela_size / rela_ent,
            )
        };
        for rela in relas {
            match rela.get_type() {
                8 /* R_AMD64_RELATIVE */ | 1027 /* R_AARCH64_RELATIVE */ => {
                    let slot = self.base + rela.get_offset() as usize;
                    let value = self.base + rela.get_addend() as usize;
                    unsafe { slot.store(value) }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn do_load(&mut self) -> Result<Address, &'static str> {
        self.map_memory()?;
        for ph in self
            .elf
            .program_iter()
            .filter(|ph| ph.get_type() == Ok(Type::Load))
        {
            self.load_segment(ph)?;
        }
        for ph in self
            .elf
            .program_iter()
            .filter(|ph| ph.get_type() == Ok(Type::Dynamic))
        {
            self.apply_relocation(ph)?;
        }
        Ok(self.base + self.elf.header.pt2.entry_point() as usize)
    }

    pub fn load(
        data: &'a [u8],
        map_pages: &'b mut dyn FnMut(usize) -> Range<Page>,
    ) -> Result<Address, &'static str> {
        Self::new(data, map_pages).do_load()
    }
}
