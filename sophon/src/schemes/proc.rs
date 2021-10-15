use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    vec,
};
use ipc::{
    scheme::{Args, Mode, Resource, Result as IoResult, SchemeServer, Uri},
    ProcId,
};
use spin::Mutex;

use crate::{kernel_tasks::user::UserTask, task::Proc};

pub struct ProcScheme {
    uris: Mutex<BTreeMap<Resource, String>>,
}

/// Process management scheme.
/// Spawn new process: `WRITE proc:/spawn`
/// Exit current process: `WRITE proc:/me/exit`
impl ProcScheme {
    pub fn new() -> Self {
        Self {
            uris: Mutex::new(BTreeMap::new()),
        }
    }
}

impl SchemeServer for ProcScheme {
    fn name(&self) -> &'static str {
        "proc"
    }
    fn open(&self, uri: &Uri, _flags: u32, _mode: Mode) -> IoResult<Resource> {
        let fd = self.allocate_resource_id();
        self.uris.lock().insert(fd, uri.path.to_string());
        Ok(fd)
    }
    fn close(self, _fd: Resource) -> IoResult<()> {
        unimplemented!()
    }
    fn read(&self, _fd: Resource, _buf: &mut [u8]) -> IoResult<usize> {
        unimplemented!()
    }
    fn write(&self, fd: Resource, buf: &[u8]) -> IoResult<()> {
        let uris = self.uris.lock();
        match uris[&fd].as_str() {
            "/spawn" => {
                let args = Args::from(buf);
                let executable_path = args.get_str().unwrap();
                let proc_id = args.get::<&mut ProcId>();
                let mut data = vec![];
                let resource = Resource::open(executable_path, 0, Mode::ReadOnly).unwrap();
                loop {
                    let mut buf = [0u8; 4096];
                    let len = resource.read(&mut buf).unwrap();
                    if len == 0 {
                        break;
                    }
                    data.extend_from_slice(&buf[..len]);
                }
                let id = Proc::spawn(box UserTask::new(data)).id;
                *proc_id = id;
                Ok(())
            }
            "/me/exit" => {
                Proc::current().exit();
                Ok(())
            }
            v => unimplemented!("{:?}", v),
        }
    }
}