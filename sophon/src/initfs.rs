use alloc::borrow::ToOwned;
use alloc::{boxed::Box, collections::BTreeMap, string::String, vec::Vec};
use core::ops::Deref;
use serde::{Deserialize, Serialize};
use spin::RwLock;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct File {
    data: Vec<u8>,
}

impl File {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
}

impl Deref for File {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Dir {
    entries: BTreeMap<String, Entry>,
}

impl Dir {
    pub fn get(&self, path: &str) -> Option<&Entry> {
        let (name, path) = match path.split_once('/') {
            Some(x) => x,
            _ => (path, ""),
        };
        if !self.entries.contains_key(name) {
            return None;
        }
        match (&self.entries[name], path) {
            (entry, "") => Some(entry),
            (Entry::Dir(dir), _) => dir.get(path),
            (_, _) => None,
        }
    }

    pub fn insert(&mut self, path: &str, file: File) {
        let (name, path) = match path.split_once('/') {
            Some(x) => x,
            _ => (path, ""),
        };
        if path.is_empty() {
            debug_assert!(!self.entries.contains_key(name));
            self.entries.insert(name.to_owned(), Entry::File(file));
        } else {
            let dir = match self.entries.get_mut(name) {
                Some(Entry::Dir(dir)) => dir,
                Some(_) => unreachable!(),
                None => {
                    self.entries
                        .insert(name.to_owned(), Entry::Dir(Dir::default()));
                    match self.entries.get_mut(name) {
                        Some(Entry::Dir(dir)) => dir,
                        _ => unreachable!(),
                    }
                }
            };
            dir.insert(path, file)
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Entry {
    File(File),
    Dir(Dir),
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct InitFS {
    root: Dir,
}

#[allow(unused)]
impl InitFS {
    pub const fn new() -> Self {
        Self {
            root: Dir {
                entries: BTreeMap::new(),
            },
        }
    }

    pub fn get_file(&self, path: &'static str) -> &File {
        debug_assert!(path.starts_with('/'));
        let path = path.strip_prefix('/').unwrap();
        match self.root.get(path) {
            Some(Entry::File(file)) => file,
            _ => unreachable!("File does not exist: {:?}", path),
        }
    }

    pub fn insert(&mut self, path: &'static str, file: File) {
        debug_assert!(path.starts_with('/'));
        let path = path.strip_prefix('/').unwrap();
        self.root.insert(path, file)
    }

    pub fn serialize(&self) -> Vec<u8> {
        postcard::to_allocvec(self).unwrap()
    }

    pub fn deserialize(buf: &[u8]) {
        let init_fs: Box<Self> = Box::new(postcard::from_bytes(buf).unwrap());
        *INIT_FS.write() = Some(Box::leak(init_fs));
    }

    pub fn get() -> &'static Self {
        INIT_FS.read().unwrap()
    }
}

static INIT_FS: RwLock<Option<&'static InitFS>> = RwLock::new(None);
