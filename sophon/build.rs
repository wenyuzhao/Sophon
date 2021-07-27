#![feature(const_btree_new)]

use std::{fs::File, io::Write};

extern crate alloc;

#[path = "./src/initfs.rs"]
mod initfs;

#[cfg(debug_assertions)]
const INIT: &'static [u8] = include_bytes!("../target/aarch64-sophon/debug/init");
#[cfg(not(debug_assertions))]
const INIT: &'static [u8] = include_bytes!("../target/aarch64-sophon/release/init");

fn main() {
    let mut init_fs = initfs::InitFS::default();
    init_fs.insert("/init", initfs::File::new(INIT.to_vec()));
    let mut init_rd = File::create("../target/_boot/init.fs").unwrap();
    init_rd.write_all(&init_fs.serialize()).unwrap();
}
