use ipc::scheme::{Mode, Resource, Result as IoResult, SchemeServer, Uri};

pub struct SystemScheme {}

impl SystemScheme {
    pub fn new() -> Self {
        Self {}
    }
}

impl SchemeServer for SystemScheme {
    fn name(&self) -> &'static str {
        "system"
    }
    fn open(&self, _uri: &Uri, _flags: u32, _mode: Mode) -> IoResult<Resource> {
        log!("SystemSchemeServer 0");
        Ok(Resource(0))
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
