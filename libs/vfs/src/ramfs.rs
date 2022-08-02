use alloc::borrow::ToOwned;
use alloc::{collections::BTreeMap, string::String, vec::Vec};
use core::ops::Deref;
use serde::{Deserialize, Serialize};

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

    pub fn entries(&self) -> Vec<String> {
        self.entries.keys().cloned().collect()
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

    pub fn mount(&mut self, path: &str, mnt: Mount) {
        let (name, path) = match path.split_once('/') {
            Some(x) => x,
            _ => (path, ""),
        };
        if path.is_empty() {
            debug_assert!(!self.entries.contains_key(name));
            self.entries.insert(name.to_owned(), Entry::Mount(mnt));
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
            dir.mount(path, mnt)
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Mount {
    pub key: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Entry {
    File(File),
    Dir(Dir),
    Mount(Mount),
}

impl Entry {
    pub fn as_file(&self) -> Option<&File> {
        match self {
            Self::File(file) => Some(file),
            _ => None,
        }
    }
    pub fn as_dir(&self) -> Option<&Dir> {
        match self {
            Self::Dir(dir) => Some(dir),
            _ => None,
        }
    }
    pub fn as_mnt(&self) -> Option<&Mount> {
        match self {
            Self::Mount(mnt) => Some(mnt),
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RamFS {
    pub root: Entry,
}

#[allow(unused)]
impl RamFS {
    pub const fn new() -> Self {
        Self {
            root: Entry::Dir(Dir {
                entries: BTreeMap::new(),
            }),
        }
    }

    pub fn get(&self, path: &str) -> Option<&Entry> {
        debug_assert!(path.starts_with('/'));
        let path = path.strip_prefix('/').unwrap();
        self.root.as_dir().unwrap().get(path)
    }

    pub fn mount(&mut self, path: &str, mnt: Mount) {
        debug_assert!(path.starts_with('/'));
        let path = path.strip_prefix('/').unwrap();
        if let Entry::Dir(dir) = &mut self.root {
            dir.mount(path, mnt)
        } else {
            unreachable!()
        }
    }

    pub fn insert(&mut self, path: &str, file: File) {
        debug_assert!(path.starts_with('/'));
        let path = path.strip_prefix('/').unwrap();
        if let Entry::Dir(dir) = &mut self.root {
            dir.insert(path, file)
        } else {
            unreachable!()
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        postcard::to_allocvec(self).unwrap()
    }

    pub fn deserialize(buf: &[u8]) -> Self {
        postcard::from_bytes(buf).unwrap()
    }
}

// static INIT_FS: RwLock<Option<&'static RamFS>> = RwLock::new(None);
