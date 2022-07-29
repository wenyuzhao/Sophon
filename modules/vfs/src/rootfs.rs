use core::{
    ptr,
    sync::atomic::{AtomicBool, Ordering},
};

use alloc::{borrow::ToOwned, boxed::Box, format, string::String, vec::Vec};
use fs::ramfs::RamFS;
use spin::{Lazy, RwLock};

use crate::fs::{FileSystem, Node, Stat};

pub static ROOT_FS: Lazy<RootFS> = Lazy::new(|| RootFS::new());

pub struct RootFS {
    ramfs: RwLock<Box<RamFS>>,
    is_initialized: AtomicBool,
}

impl RootFS {
    pub fn new() -> Self {
        RootFS {
            ramfs: RwLock::new(Box::new(RamFS::new())),
            is_initialized: AtomicBool::new(false),
        }
    }

    fn is_initialized(&self) -> bool {
        self.is_initialized.load(Ordering::SeqCst)
    }

    pub fn init(&self, ramfs: &'static mut RamFS) {
        if self
            .is_initialized
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            != Ok(false)
        {
            return;
        }
        assert!(ramfs.get("/etc").is_some());
        *self.ramfs.write() = unsafe { Box::from_raw(ramfs) };
        assert!(self.is_initialized())
    }

    pub fn root_node(&self) -> Node {
        Node {
            name: "/".into(),
            path: "".into(),
            fs: unsafe { &*(self as *const Self) },
            mount: None,
            block: 0,
            offset: 0,
        }
    }
}

impl FileSystem for RootFS {
    fn stat(&self, parent: &Node, file: &str) -> Option<Stat> {
        let fs = self.ramfs.read();
        assert!(self.is_initialized());
        fs.get(&format!("{}/{}", parent.path, file))
            .map(|entry| Stat {
                fs: unsafe { &*(self as *const Self) },
                mount: false,
                is_dir: entry.as_dir().is_some(),
            })
    }
    fn open(&self, parent: &Node, fname: &str) -> Option<Node> {
        let fs = self.ramfs.read();
        let path = format!("{}/{}", parent.path, fname);
        fs.get(&path)
            .map(|entry| entry.as_file())
            .flatten()
            .map(|_| Node {
                name: fname.to_owned().into(),
                path: path.into(),
                fs: unsafe { &*(self as *const Self) },
                mount: None,
                block: 0,
                offset: 0,
            })
    }
    fn read(&self, node: &Node, offset: usize, buf: &mut [u8]) -> Option<usize> {
        let fs = self.ramfs.read();
        if let Some(file) = fs.get(&node.path).map(|entry| entry.as_file()).flatten() {
            if offset >= file.len() {
                return Some(0);
            }
            let start = offset;
            let end = usize::min(file.len(), offset + buf.len());
            let bytes = end - start;
            unsafe {
                ptr::copy_nonoverlapping::<u8>(
                    file[start..end].as_ptr(),
                    buf[..bytes].as_mut_ptr(),
                    bytes,
                )
            }
            Some(bytes)
        } else {
            None
        }
    }
    fn read_dir(&self, _node: &Node) -> Option<Vec<String>> {
        unimplemented!()
    }
}
