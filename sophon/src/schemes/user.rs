use alloc::string::String;
use ipc::{
    scheme::{Mode, Resource, Result as IoResult, SchemeServer, Uri},
    TaskId,
};

pub struct UserScheme {
    name: String,
    pub handler: TaskId,
}

impl UserScheme {
    pub fn new(name: String, handler: TaskId) -> Self {
        Self { name, handler }
    }
}

impl SchemeServer for UserScheme {
    fn name(&self) -> &str {
        &self.name
    }
    fn open(&self, _uri: &Uri, _flags: u32, _mode: Mode) -> IoResult<Resource> {
        unimplemented!()
    }
    fn close(self, _fd: Resource) -> IoResult<()> {
        unimplemented!()
    }
    fn read(&self, _fd: Resource, buf: &mut [u8]) -> IoResult<usize> {
        let msg = "test\0".as_bytes();
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
    fn write(&self, _fd: Resource, buf: &[u8]) -> IoResult<()> {
        let s = core::str::from_utf8(buf).unwrap();
        log!("{}", s);
        Ok(())
    }
}
