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

fn vfs_locate_node<'a>(parent: &Node, path: &'a str) -> Option<(Node, &'a str)> {
    assert!(!path.starts_with("/"));
    let (entry, remaining_path) = path.split_once("/").unwrap_or_else(|| (path, ""));
    let stat = parent.fs.stat(parent, entry)?;
    if stat.is_dir {
        if remaining_path == "" {
            Some((parent.clone(), entry))
        } else {
            vfs_locate_node(
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
        vfs_locate_node(&mnt.root, remaining_path)
    } else {
        Some((parent.clone(), entry))
    }
}

pub fn vfs_open(path: &str) -> Option<Node> {
    assert!(path.starts_with("/"));
    if path == "/" {
        return None;
    }
    let (parent, entry) = vfs_locate_node(&ROOT_FS.root_node(), path.split_once("/").unwrap().1)?;
    parent.fs.open(&parent, entry)
}
