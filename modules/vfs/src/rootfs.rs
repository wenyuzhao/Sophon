use core::{
    ptr,
    sync::atomic::{AtomicBool, Ordering},
};

use alloc::{borrow::ToOwned, boxed::Box, format, string::String, vec::Vec};
use spin::{Lazy, RwLock};
use vfs::ramfs::{self, RamFS};
use vfs::{FileSystem, Node, Stat};

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

    pub fn init(&'static self, ramfs: &'static mut RamFS) {
        if self
            .is_initialized
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            != Ok(false)
        {
            return;
        }
        assert!(ramfs.get("/etc").is_some());
        *self.ramfs.write() = unsafe { Box::from_raw(ramfs) };
        assert!(self.is_initialized());
        crate::FILE_SYSTEMS
            .write()
            .insert("rootfs".to_owned(), self);
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
    fn name(&self) -> &'static str {
        "rootfs"
    }
    fn stat(&self, parent: &Node, file: &str) -> Option<Stat> {
        let fs = self.ramfs.read();
        assert!(self.is_initialized());
        fs.get(&format!("{}/{}", parent.path, file))
            .map(|entry| Stat {
                fs: unsafe { &*(self as *const Self) },
                mount: entry.as_mnt().map(|x| x.key),
                is_dir: entry.as_dir().is_some(),
            })
    }
    fn open(&self, parent: &Node, fname: &str) -> Option<Node> {
        let fs = self.ramfs.read();
        let path = format!("{}/{}", parent.path, fname);
        fs.get(&path).map(|e| Node {
            name: fname.to_owned().into(),
            path: path.into(),
            fs: unsafe { &*(self as *const Self) },
            mount: e.as_mnt().map(|x| x.key),
            block: 0,
            offset: 0,
        })
    }
    fn close(&self, _node: &Node) {
        // Nothing to do
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
    fn write(&self, _node: &Node, _offset: usize, _buf: &[u8]) -> Option<usize> {
        unimplemented!()
    }
    fn read_dir(&self, node: &Node) -> Option<Vec<String>> {
        let fs = self.ramfs.read();
        let path = if node.path.is_empty() {
            "/"
        } else {
            &node.path
        };
        if let Some(dir) = fs.get(path).map(|entry| entry.as_dir()).flatten() {
            Some(dir.entries())
        } else {
            None
        }
    }
    fn mount(&self, parent: &Node, file: &str, key: usize) -> Option<Node> {
        let mut fs = self.ramfs.write();
        let path = format!("{}/{}", parent.path, file);
        println!("mount {}", path);
        fs.mount(&path, ramfs::Mount { key }).ok()?;
        Some(Node {
            name: file.to_owned().into(),
            path: path.into(),
            fs: unsafe { &*(self as *const Self) },
            mount: Some(key),
            block: 0,
            offset: 0,
        })
    }
}

#[test]
fn root_ramfs_read_text_file() {
    let fs = ROOT_FS.ramfs.read();
    let file = fs.get("/etc/hello.txt").unwrap().as_file().unwrap();
    let s = core::str::from_utf8(file).unwrap();
    assert_eq!(s, "Hello world from file!");
}
