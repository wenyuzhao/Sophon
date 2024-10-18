use crate::rootfs::ROOT_FS;
use alloc::{
    borrow::{Cow, ToOwned},
    format,
};
use spin::RwLock;
use vfs::{FileSystem, Node};

// static MOUNT_POINTS: BTreeMap<>
#[allow(unused)]
pub struct MountPoint {
    pub parent: Node,
    pub root: Node,
    pub dev: usize,
    pub fs: &'static dyn FileSystem,
}

pub static MOUNT_POINTS: RwLock<[Option<MountPoint>; 32]> = RwLock::new([
    None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None, None, None, None, None, None, None, None, None, None,
]);

pub fn vfs_mount(path: &str, dev: usize, fs: &'static dyn FileSystem) -> Option<Node> {
    assert!(path.starts_with("/"));
    if path == "/" {
        return None;
    }
    let mut mount_points = MOUNT_POINTS.write();
    let mut i = 0;
    while i < mount_points.len() {
        if mount_points[i].is_none() {
            let (parent, root) = vfs_mount_impl(
                &ROOT_FS.root_node(),
                path.split_once("/").unwrap().1,
                dev,
                fs,
                i,
            )?;
            mount_points[i] = Some(MountPoint {
                parent,
                root: root.clone(),
                dev,
                fs,
            });
            return Some(root);
        }
        i += 1;
    }
    None
}

fn vfs_mount_impl(
    parent: &Node,
    path: &str,
    dev: usize,
    fs: &'static dyn FileSystem,
    key: usize,
) -> Option<(Node, Node)> {
    assert!(!path.starts_with("/"));
    let (entry, remaining_path) = path.split_once("/").unwrap_or_else(|| (path, ""));
    if let Some(stat) = parent.fs.stat(parent, entry) {
        if stat.is_dir {
            if remaining_path == "" {
                warn!("{} is a directory", path);
                None
            } else {
                vfs_mount_impl(
                    &Node {
                        name: Cow::Owned(entry.to_owned()),
                        path: Cow::Owned(format!("{}/{}", parent.path, entry)),
                        fs: parent.fs,
                        mount: parent.mount,
                        block: parent.block,
                        offset: parent.offset,
                    },
                    remaining_path,
                    dev,
                    fs,
                    key,
                )
            }
        } else {
            let parent = parent.fs.mount(parent, entry, key)?;
            let mut root = parent.clone();
            root.mount = None;
            root.fs = fs;
            Some((parent, root))
        }
    } else if remaining_path == "" {
        let parent = parent.fs.mount(parent, entry, key)?;
        let mut root = parent.clone();
        root.mount = None;
        root.fs = fs;
        Some((parent, root))
    } else {
        None
    }
}
