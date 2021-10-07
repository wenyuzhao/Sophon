#![feature(asm)]
#![feature(format_args_nl)]
#![feature(default_alloc_error_handler)]
#![no_std]
#![no_main]

use heap::NoAlloc;
use ipc::log;
use ipc::scheme::{self, Mode, Resource, Result as IoResult, SchemeServer, Uri};

#[global_allocator]
static ALLOCATOR: NoAlloc = NoAlloc;

struct ExampleUserSchemeServer {}

impl SchemeServer for ExampleUserSchemeServer {
    fn name(&self) -> &'static str {
        "scheme-test"
    }
    fn open(&self, _uri: &Uri, _flags: u32, _mode: Mode) -> IoResult<Resource> {
        Ok(Resource(1))
    }
    fn close(self, _fd: Resource) -> IoResult<()> {
        unimplemented!()
    }
    fn read(&self, _fd: Resource, buf: &mut [u8]) -> IoResult<usize> {
        let msg = "UserSchemeServer".as_bytes();
        let mut cursor = 0;
        for b in msg {
            if cursor >= buf.len() {
                break;
            }
            buf[cursor] = *b;
            cursor += 1;
        }
        Ok(cursor)
    }
    fn write(&self, _fd: Resource, _buf: &[u8]) -> IoResult<()> {
        unimplemented!()
    }
}

#[no_mangle]
pub extern "C" fn _start(_argc: isize, _argv: *const *const u8) -> isize {
    log!("scheme_test start (user mode)");
    scheme::register_user_scheme(&ExampleUserSchemeServer {})
}

#[panic_handler]
fn panic(info: &::core::panic::PanicInfo) -> ! {
    log!("{}", info);
    loop {}
}
