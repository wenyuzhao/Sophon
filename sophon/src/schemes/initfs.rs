use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
};
use ipc::scheme::{Error, Mode, Resource, Result as IoResult, SchemeServer, Uri};
use spin::Mutex;

use crate::initfs::InitFS;

struct FileState {
    path: String,
    cursor: usize,
}

pub struct InitFSScheme {
    fd_map: Mutex<BTreeMap<Resource, FileState>>,
}

impl InitFSScheme {
    pub fn new() -> Self {
        Self {
            fd_map: Mutex::new(BTreeMap::new()),
        }
    }
}

impl SchemeServer for InitFSScheme {
    fn name(&self) -> &'static str {
        "init"
    }
    fn open(&self, uri: &Uri, _flags: u32, _mode: Mode) -> IoResult<Resource> {
        let fd = self.allocate_resource_id();
        self.fd_map.lock().insert(
            fd,
            FileState {
                path: uri.path.to_string(),
                cursor: 0,
            },
        );
        Ok(fd)
    }
    fn close(self, _fd: Resource) -> IoResult<()> {
        unimplemented!()
    }
    fn read(&self, fd: Resource, buf: &mut [u8]) -> IoResult<usize> {
        let mut fd_map = self.fd_map.lock();
        let file_state = fd_map.get_mut(&fd).unwrap();
        let file_data: &[u8] = &InitFS::get().get_file(&file_state.path);
        let mut file_cursor = file_state.cursor;
        let mut buf_cursor = 0;
        while file_cursor < file_data.len() && buf_cursor < buf.len() {
            buf[buf_cursor] = file_data[file_cursor];
            buf_cursor += 1;
            file_cursor += 1;
        }
        file_state.cursor = file_cursor;
        Ok(buf_cursor)
    }
    fn write(&self, _fd: Resource, _buf: &[u8]) -> IoResult<()> {
        Err(Error::Other)
    }
}
