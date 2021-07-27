#![feature(const_btree_new)]

extern crate alloc;

use std::{
    fs::{self, File},
    io::{BufReader, Read, Write},
    path::Path,
};
use yaml_rust::YamlLoader;

mod initfs {
    include!(concat!(env!("OUT_DIR"), "/init-fs.rs"));
}

include!(concat!(env!("OUT_DIR"), "/profile.rs"));

fn main() {
    // Create ram fs
    let mut init_fs = initfs::InitFS::default();
    // Add files
    let docs =
        YamlLoader::load_from_str(&fs::read_to_string("../../user/Build.yml").unwrap()).unwrap();
    let config = &docs[0];
    let init_fs_files = config["init.fs"].as_hash().unwrap();
    for (name, path) in init_fs_files
        .iter()
        .map(|(k, v)| (k.as_str().unwrap(), v.as_str().unwrap()))
    {
        let file = File::open(format!("../../target/aarch64-sophon/{}/{}", PROFILE, name)).unwrap();
        let mut buffer = Vec::new();
        BufReader::new(file).read_to_end(&mut buffer).unwrap();
        init_fs.insert(path, initfs::File::new(buffer));
    }
    // Serialize
    fs::create_dir_all(Path::new("../../target/_boot")).unwrap();
    let mut init_rd = File::create("../../target/_boot/init.fs").unwrap();
    init_rd.write_all(&init_fs.serialize()).unwrap();
}
