#![no_std]
#![feature(step_trait)]
#![feature(format_args_nl)]

#[allow(unused_imports)]
#[macro_use]
extern crate log;

use core::ops::Range;

use memory::address::{Address, V};
use memory::page::{Page, PageSize, Size4K};
use xmas_elf::{
    dynamic,
    program::{ProgramHeader, SegmentData, Type},
    sections::Rela,
    ElfFile,
};

pub struct ELFLoader<'a, 'b, 'c> {
    data: &'a [u8],
    elf: ElfFile<'a>,
    vaddr_offset: isize,
    map_pages: &'b mut dyn FnMut(Range<Page>) -> Range<Page>,
    translate: Option<&'c dyn Fn(Address) -> Address>,
}

impl<'a, 'b, 'c> ELFLoader<'a, 'b, 'c> {
    fn new(
        data: &'a [u8],
        map_pages: &'b mut dyn FnMut(Range<Page>) -> Range<Page>,
        translate: Option<&'c dyn Fn(Address) -> Address>,
    ) -> Self {
        ELFLoader {
            data: data,
            elf: ElfFile::new(data).unwrap(),
            vaddr_offset: 0,
            map_pages,
            translate,
        }
    }

    fn addr(&self, a: Address) -> Address {
        self.translate.as_ref().map(|f| (*f)(a)).unwrap_or(a)
    }

    fn copy(&self, src: &[u8], dst: Address) {
        if src.is_empty() {
            return;
        }
        if let Some(translate) = self.translate.as_ref() {
            let mut cursor = dst;
            let limit = dst + src.len();
            while cursor < limit {
                let start = translate(cursor);
                let bytes =
                    Address::min(Page::<Size4K>::align(cursor) + Size4K::BYTES, limit) - cursor;
                unsafe {
                    core::ptr::copy_nonoverlapping::<u8>(
                        &src[cursor - dst],
                        start.as_mut_ptr(),
                        bytes,
                    );
                }
                cursor += bytes;
            }
        } else {
            unsafe {
                core::ptr::copy_nonoverlapping::<u8>(&src[0], dst.as_mut_ptr(), src.len());
            }
        }
    }

    fn zero(&self, dst: Address, bytes: usize) {
        if bytes == 0 {
            return;
        }
        if let Some(translate) = self.translate.as_ref() {
            let mut cursor = dst;
            let limit = dst + bytes;
            while cursor < limit {
                let start = translate(cursor);
                let bytes =
                    Address::min(Page::<Size4K>::align(cursor) + Size4K::BYTES, limit) - cursor;
                unsafe {
                    core::ptr::write_bytes::<u8>(start.as_mut_ptr(), 0, bytes);
                }
                cursor += bytes;
            }
        } else {
            unsafe {
                core::ptr::write_bytes::<u8>(dst.as_mut_ptr(), 0, bytes);
            }
        }
    }

    fn flush(&self, dst: Address, bytes: usize) {
        if bytes == 0 {
            return;
        }
        if let Some(translate) = self.translate.as_ref() {
            let mut cursor = dst;
            let limit = dst + bytes;
            while cursor < limit {
                let start = translate(cursor);
                let bytes =
                    Address::min(Page::<Size4K>::align(cursor) + Size4K::BYTES, limit) - cursor;
                memory::cache::flush_cache(start..start + bytes);
                cursor += bytes;
            }
        } else {
            memory::cache::flush_cache(dst..dst + bytes);
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
        let vaddr_start = Page::<Size4K>::align(load_start.unwrap());
        let vaddr_end = load_end.unwrap().align_up(Size4K::BYTES);
        // log!("vaddr: {:?} .. {:?}", vaddr_start, vaddr_end);
        let pages =
            (self.map_pages)(Page::<Size4K>::new(vaddr_start)..Page::<Size4K>::new(vaddr_end));
        self.vaddr_offset =
            pages.start.start().as_usize() as isize - vaddr_start.as_usize() as isize;
        Ok(())
    }

    fn load_segment(&self, ph: ProgramHeader) -> Result<(), &'static str> {
        // Copy data
        let start: Address = Address::from(ph.virtual_addr() as usize) + self.vaddr_offset;
        let bytes = ph.file_size() as usize;
        let offset = ph.offset() as usize;
        // log!("copy: dst={:?}", start..(start + bytes));
        self.copy(&self.data[offset..offset + bytes], start);
        // Zero data
        if ph.mem_size() > ph.file_size() {
            let zero_start = start + ph.file_size() as usize;
            let zero_end = start + ph.mem_size() as usize;
            // log!("zero: dst={:?}", zero_start..zero_end);
            self.zero(zero_start, zero_end - zero_start);
        }
        // Flush cache
        // log!("flush");
        self.flush(start, bytes);
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
                    let slot = self.addr(Address::from(rela.get_offset() as usize)+ self.vaddr_offset);
                    let value = Address::<V>::from(rela.get_addend() as usize) + self.vaddr_offset;
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
            // log!("Load {:?}", ph);
            self.load_segment(ph)?;
        }
        for ph in self
            .elf
            .program_iter()
            .filter(|ph| ph.get_type() == Ok(Type::Dynamic))
        {
            // log!("Relo {:?}", ph);
            self.apply_relocation(ph)?;
        }
        Ok(Address::from(self.elf.header.pt2.entry_point() as usize) + self.vaddr_offset)
    }

    pub fn load(
        data: &'a [u8],
        map_pages: &'b mut dyn FnMut(Range<Page>) -> Range<Page>,
    ) -> Result<Address, &'static str> {
        ELFLoader::new(data, map_pages, None).do_load()
    }

    pub fn load_with_address_translation(
        data: &'a [u8],
        map_pages: &'b mut dyn FnMut(Range<Page>) -> Range<Page>,
        translate: &'c dyn Fn(Address) -> Address,
    ) -> Result<Address, &'static str> {
        ELFLoader::new(data, map_pages, Some(translate)).do_load()
    }
}
