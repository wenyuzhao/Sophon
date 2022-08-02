use alloc::{
    borrow::{Cow, ToOwned},
    format,
};
use vfs::Node;

use crate::rootfs::ROOT_FS;

/// Per-process file descriptor
pub struct FileDescriptor {
    pub node: Node,
    pub offset: usize,
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
    } else if let Some(mnt) = stat.mount {
        let mnt_table = super::mount::MOUNT_POINTS.read();
        let mnt = mnt_table[mnt].as_ref()?;
        vfs_open_impl(&mnt.root, remaining_path)
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
