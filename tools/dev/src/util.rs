use std::{fs, path::Path};

use xshell::Cmd;
use yaml_rust::{Yaml, YamlLoader};

#[derive(Clap, Clone)]
pub struct CargoFlags {
    #[clap(default_value = "aarch64")]
    pub arch: String,
    pub features: Option<String>,
    #[clap(long = "release")]
    pub release: bool,
}

impl CargoFlags {
    /// Return: (target_name, target_path)
    pub fn user_traget(&self) -> (String, String) {
        assert_eq!(self.arch, "aarch64");
        let target_name = format!("{}-sophon", self.arch);
        let target_path = format!("../../sophon/{}.json", target_name);
        (target_name, target_path)
    }

    pub fn kernel_target(&self) -> &'static str {
        assert_eq!(self.arch, "aarch64");
        "aarch64-unknown-none"
    }

    pub fn uefi_target(&self) -> &'static str {
        assert_eq!(self.arch, "aarch64");
        "aarch64-uefi.json"
    }
}

pub fn copy_file(from: impl AsRef<Path>, to: impl AsRef<Path>) {
    xshell::cp(from, to).unwrap();
}

pub fn mkdir(path: impl AsRef<Path>) {
    xshell::mkdir_p(path).unwrap();
}

fn append_cargo_args(
    mut cmd: Cmd,
    package: &str,
    features: Option<String>,
    release: bool,
    target: Option<&str>,
) -> Cmd {
    cmd = cmd.args(["--package", package]);
    if let Some(features) = features {
        cmd = cmd.args(["--features", &features]);
    }
    if release {
        cmd = cmd.args(["--release"]);
    }
    if let Some(target) = target {
        cmd = cmd.args(["--target", &target]);
    }
    cmd
}

pub fn disassemble(bin: impl AsRef<Path>, out: impl AsRef<Path>) {
    let dissam = cmd!("llvm-objdump")
        .args([
            "--section-headers",
            "--source",
            "-d",
            bin.as_ref().to_str().unwrap(),
        ])
        .ignore_stderr()
        .read()
        .unwrap();
    fs::write(out, dissam).unwrap();
}

pub fn build_package(
    name: impl AsRef<str>,
    path: impl AsRef<Path>,
    features: Option<String>,
    release: bool,
    target: Option<&str>,
) {
    let _p = xshell::pushd(path).unwrap();
    let mut cmd = cmd!("cargo build");
    cmd = append_cargo_args(cmd, name.as_ref(), features, release, target);
    cmd.run().unwrap();
}

pub fn run_package(
    name: &str,
    path: impl AsRef<Path>,
    features: Option<String>,
    release: bool,
    target: Option<&str>,
) {
    let _p = xshell::pushd(path).unwrap();
    let mut cmd = cmd!("cargo run");
    cmd = append_cargo_args(cmd, name, features, release, target);
    cmd.run().unwrap();
}

pub fn load_yaml(path: impl AsRef<Path>) -> Vec<Yaml> {
    YamlLoader::load_from_str(&fs::read_to_string(path).unwrap()).unwrap()
}
