use std::{env, fs, io, path::Path};

fn main() -> io::Result<()> {
    let out_dir = env::var_os("OUT_DIR").unwrap();

    fs::copy(
        "../../sophon/init-fs.rs",
        Path::new(&out_dir).join("init-fs.rs"),
    )?;

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../../sophon/init-fs.rs");

    Ok(())
}
