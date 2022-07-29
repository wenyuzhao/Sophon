use alloc::{
    borrow::{Cow, ToOwned},
    format,
    string::String,
    vec::Vec,
};

use crate::{mount::Mount, rootfs::ROOT_FS};

/// Per-process file descriptor
pub struct FileDescriptor {
    pub node: Node,
    pub offset: usize,
}

#[derive(Clone)]
pub struct Node {
    pub name: Cow<'static, str>,
    pub path: Cow<'static, str>,
    pub fs: &'static dyn FileSystem,
    pub mount: Option<&'static Mount>,
    pub block: usize,
    pub offset: usize,
}

pub struct Stat {
    pub fs: &'static dyn FileSystem,
    pub mount: bool,
    pub is_dir: bool,
}

pub trait FileSystem: Sync + Send {
    fn stat(&self, parent: &Node, file: &str) -> Option<Stat>;
    // File operations
    fn open(&self, parent: &Node, file: &str) -> Option<Node>;
    fn read(&self, node: &Node, offset: usize, buf: &mut [u8]) -> Option<usize>;
    // Dir operations
    fn read_dir(&self, node: &Node) -> Option<Vec<String>>;
}

fn vfs_open_impl(parent: &Node, path: &str) -> Option<Node> {
    assert!(!path.starts_with("/"));
    let (entry, remaining_path) = path.split_once("/").unwrap_or_else(|| (path, ""));
    let stat = parent.fs.stat(parent, entry)?;
    if stat.is_dir {
        if remaining_path == "" {
            None
        } else {
            vfs_open_impl(
                &Node {
                    name: Cow::Owned(entry.to_owned()),
                    path: Cow::Owned(format!("{}/{}", parent.path, entry)),
                    fs: parent.fs,
                    mount: parent.mount,
                    block: parent.block,
                    offset: parent.offset,
                },
                remaining_path,
            )
        }
    } else {
        parent.fs.open(parent, entry)
    }
}

pub fn vfs_open(path: &str) -> Option<Node> {
    assert!(path.starts_with("/"));
    if path == "/" {
        return None;
    }
    vfs_open_impl(&ROOT_FS.root_node(), path.split_once("/").unwrap().1)
}
