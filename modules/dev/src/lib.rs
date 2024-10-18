#![feature(format_args_nl)]
#![no_std]

#[allow(unused)]
#[macro_use]
extern crate log;
extern crate alloc;

use alloc::{borrow::ToOwned, collections::BTreeMap, format, string::String, vec::Vec};
use dev::{DevRequest, Device};
use kernel_module::{kernel_module, KernelModule, SERVICE};
use spin::{Lazy, RwLock};
use vfs::{FileSystem, Node, Stat, VFSRequest};

#[kernel_module]
pub static DEV: DEV = DEV {};

pub struct DEV {}

impl KernelModule for DEV {
    type ModuleRequest<'a> = DevRequest<'a>;

    fn init(&mut self) -> anyhow::Result<()> {
        SERVICE.vfs().register_fs(&*DEV_FS);
        kernel_module::module_call(
            "vfs",
            &VFSRequest::Mount {
                path: "/dev",
                dev: 0,
                fs: "devfs",
            },
        );
        // Mount dev-fs
        Ok(())
    }

    fn handle_module_call<'a>(&self, privileged: bool, request: Self::ModuleRequest<'a>) -> isize {
        match request {
            DevRequest::RegisterDev(dev) => {
                assert!(privileged);
                DEV_FS.devices.write().insert(dev.name().to_owned(), *dev);
                0
            }
        }
    }
}

pub static DEV_FS: Lazy<DevFS> = Lazy::new(|| DevFS::new());

pub struct DevFS {
    devices: RwLock<BTreeMap<String, &'static dyn Device>>,
}

impl DevFS {
    pub fn new() -> Self {
        DevFS {
            devices: RwLock::new(BTreeMap::new()),
        }
    }
}

impl FileSystem for DevFS {
    fn name(&self) -> &'static str {
        "devfs"
    }
    fn stat(&self, parent: &Node, file: &str) -> Option<Stat> {
        if parent.path != "/dev" {
            return None;
        }
        let devices = self.devices.read();
        if !devices.contains_key(file) {
            return None;
        }
        Some(Stat {
            fs: unsafe { &*(self as *const Self) },
            mount: None,
            is_dir: false,
        })
    }
    fn open(&self, parent: &Node, fname: &str) -> Option<Node> {
        let devices = self.devices.read();
        if !devices.contains_key(fname) {
            return None;
        }
        let path = format!("{}/{}", parent.path, fname);
        Some(Node {
            name: fname.to_owned().into(),
            path: path.into(),
            fs: unsafe { &*(self as *const Self) },
            mount: None,
            block: 0,
            offset: 0,
        })
    }
    fn close(&self, _node: &Node) {
        // Nothing to do
    }
    fn read(&self, node: &Node, offset: usize, buf: &mut [u8]) -> Option<usize> {
        let devices = self.devices.read();
        if !devices.contains_key(node.name.as_ref()) {
            return None;
        }
        devices[node.name.as_ref()].read(offset, buf)
    }
    fn write(&self, node: &Node, offset: usize, buf: &[u8]) -> Option<usize> {
        let devices = self.devices.read();
        if !devices.contains_key(node.name.as_ref()) {
            return None;
        }
        devices[node.name.as_ref()].write(offset, buf)
    }
    fn read_dir(&self, node: &Node) -> Option<Vec<String>> {
        if node.path.as_ref() == "/dev" {
            let devices = self.devices.read();
            Some(devices.keys().cloned().collect())
        } else {
            None
        }
    }
    fn mount(&self, _parent: &Node, _file: &str, _key: usize) -> Option<Node> {
        unimplemented!()
    }
}
