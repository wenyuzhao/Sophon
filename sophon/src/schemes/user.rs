use crate::arch::ArchContext;
use crate::memory::kernel::KERNEL_MEMORY_MAPPER;
use crate::{
    memory::{kernel::KERNEL_HEAP, physical::PHYSICAL_MEMORY},
    task::Task,
};
use alloc::string::String;
use core::iter::Step;
use core::{intrinsics::transmute, ops::Range};
use core::{ptr, slice, str};
use ipc::{
    scheme::{Error, Mode, Resource, Result as IoResult, SchemeRequest, SchemeServer, Uri},
    Message, TaskId,
};
use memory::page::{Frame, Page, PageSize, Size4K};
use memory::page_table::{PageFlags, PageFlagsExt};

pub struct UserScheme {
    name: String,
    pub handler: TaskId,
}

impl UserScheme {
    pub fn new(name: String, handler: TaskId) -> Self {
        Self { name, handler }
    }
    // Allocate pages that is mapped in both kernel and handler's address space.
    fn map_handler_pages(&self, num_pages: usize) -> (Range<Page>, Range<Page>) {
        let handler = Task::by_id(self.handler).unwrap();
        let handler_pages = handler.sbrk(num_pages).unwrap();
        let kernel_pages = KERNEL_HEAP.virtual_allocate::<Size4K>(num_pages);
        let handler_page_table = handler.context.get_page_table();
        let _guard = KERNEL_MEMORY_MAPPER.with_kernel_page_table();
        for (i, page) in handler_pages.clone().enumerate() {
            let frame = Frame::<Size4K>::new(handler_page_table.translate(page.start()).unwrap());
            handler_page_table.map(
                Page::forward(kernel_pages.start, i),
                frame,
                PageFlags::kernel_data_flags_4k(),
                &PHYSICAL_MEMORY,
            );
        }
        (kernel_pages, handler_pages)
    }

    fn unmap_handler_pages(&self, _kernel_pages: Range<Page>, _handler_pages: Range<Page>) {
        // FIXME: unimplemented
    }
}

impl SchemeServer for UserScheme {
    fn name(&self) -> &str {
        &self.name
    }
    fn open(&self, uri: &Uri, flags: u32, mode: Mode) -> IoResult<Resource> {
        // Copy uri string
        let s = uri.raw.as_bytes();
        let len = s.len();
        let num_pages = (s.len() + Size4K::MASK) >> Size4K::LOG_BYTES;
        let (kernel_pages, handler_pages) = self.map_handler_pages(num_pages);
        unsafe {
            let kernel_buf: &mut [u8] =
                slice::from_raw_parts_mut(kernel_pages.start.start().as_mut_ptr(), len);
            ptr::copy_nonoverlapping::<u8>(s.as_ptr(), kernel_buf.as_mut_ptr(), len);
        }
        let handler_buf: &mut [u8] =
            unsafe { slice::from_raw_parts_mut(handler_pages.start.start().as_mut_ptr(), len) };
        // Call handler
        unsafe {
            Message::new(TaskId::NULL, self.handler)
                .with_data::<[usize; 5]>([
                    transmute(SchemeRequest::Open),
                    transmute(handler_buf.as_ptr()),
                    transmute(len),
                    transmute(flags as usize),
                    transmute(mode),
                ])
                .send();
        }
        let result = ipc::syscall::receive(Some(self.handler));
        let return_code = *result.get_data::<isize>();
        if return_code < 0 {
            Err(Error::Other)
        } else {
            Ok(Resource(return_code as _))
        }
    }
    fn close(self, _fd: Resource) -> IoResult<()> {
        unimplemented!()
    }
    fn read(&self, fd: Resource, buf: &mut [u8]) -> IoResult<usize> {
        // Construct new buffer
        let num_pages = (buf.len() + Size4K::MASK) >> Size4K::LOG_BYTES;
        let (kernel_pages, handler_pages) = self.map_handler_pages(num_pages);
        let len = buf.len();
        let handler_buf: &mut [u8] =
            unsafe { slice::from_raw_parts_mut(handler_pages.start.start().as_mut_ptr(), len) };
        // Call handler
        unsafe {
            Message::new(TaskId::NULL, self.handler)
                .with_data::<[usize; 5]>([
                    transmute(SchemeRequest::Read),
                    transmute(fd),
                    transmute(handler_buf.as_mut_ptr()),
                    transmute(len),
                    0,
                ])
                .send();
        }
        let result = ipc::syscall::receive(Some(self.handler));
        let return_code = *result.get_data::<isize>();
        // Copy data back
        unsafe {
            let kernel_buf: &mut [u8] =
                slice::from_raw_parts_mut(kernel_pages.start.start().as_mut_ptr(), len);
            ptr::copy_nonoverlapping::<u8>(kernel_buf.as_ptr(), buf.as_mut_ptr(), len);
        }
        self.unmap_handler_pages(kernel_pages, handler_pages);
        if return_code < 0 {
            Err(Error::Other)
        } else {
            Ok(return_code as _)
        }
    }
    fn write(&self, fd: Resource, buf: &[u8]) -> IoResult<()> {
        // Copy buffer
        let num_pages = (buf.len() + Size4K::MASK) >> Size4K::LOG_BYTES;
        let (kernel_pages, handler_pages) = self.map_handler_pages(num_pages);
        let len = buf.len();
        unsafe {
            let kernel_buf: &mut [u8] =
                slice::from_raw_parts_mut(kernel_pages.start.start().as_mut_ptr(), len);
            ptr::copy_nonoverlapping::<u8>(buf.as_ptr(), kernel_buf.as_mut_ptr(), len);
        }
        let handler_buf: &mut [u8] =
            unsafe { slice::from_raw_parts_mut(handler_pages.start.start().as_mut_ptr(), len) };
        // Call handler
        unsafe {
            Message::new(TaskId::NULL, self.handler)
                .with_data::<[usize; 5]>([
                    transmute(SchemeRequest::Write),
                    transmute(fd),
                    transmute(handler_buf.as_ptr()),
                    transmute(len),
                    0,
                ])
                .send();
        }
        let result = ipc::syscall::receive(Some(self.handler));
        let return_code = *result.get_data::<isize>();

        self.unmap_handler_pages(kernel_pages, handler_pages);
        if return_code < 0 {
            Err(Error::Other)
        } else {
            Ok(())
        }
    }
}
