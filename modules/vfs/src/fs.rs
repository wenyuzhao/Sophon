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

pub fn vfs_locate_node<'a>(parent: &Node, path: &'a str) -> Option<(Node, &'a str)> {
    assert!(!path.starts_with("/"));
    let (entry, remaining_path) = path.split_once("/").unwrap_or_else(|| (path, ""));
    let stat = parent.fs.stat(parent, entry)?;
    if remaining_path == "" {
        return Some((parent.clone(), entry));
    }
    if stat.is_dir {
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
    } else if let Some(mnt) = stat.mount {
        let mnt_table = super::mount::MOUNT_POINTS.read();
        let mnt = mnt_table[mnt].as_ref()?;
        vfs_locate_node(&mnt.root, remaining_path)
    } else {
        Some((parent.clone(), entry))
    }
}

pub fn vfs_locate_node_from_path<'a>(path: &'a str) -> Option<(Node, &'a str)> {
    assert!(path.starts_with("/"));
    let mut path = path.trim();
    if path.ends_with("/") {
        path = &path[..path.len() - 1];
    }
    if path.starts_with("/") {
        path = &path[1..];
    }
    vfs_locate_node(&ROOT_FS.root_node(), path)
}

pub fn vfs_open(path: &str) -> Option<Node> {
    if !path.starts_with("/") {
        return None;
    }
    if path == "/" {
        return Some(ROOT_FS.root_node());
    }
    let mut path = path.trim();
    if path.ends_with("/") {
        path = &path[..path.len() - 1];
    }
    if path.starts_with("/") {
        path = &path[1..];
    }
    let (parent, entry) = vfs_locate_node(&ROOT_FS.root_node(), path)?;
    parent.fs.open(&parent, entry)
}

pub fn dir_or_mnt_exists(path: &str) -> bool {
    let (parent, entry) = match vfs_locate_node_from_path(path) {
        Some(x) => x,
        _ => return false,
    };
    let stat = match parent.fs.stat(&parent, entry) {
        Some(x) => x,
        _ => return false,
    };
    stat.is_dir || stat.mount.is_some()
}
