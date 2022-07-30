use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use xshell::{Cmd, Shell};
use yaml_rust::{Yaml, YamlLoader};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Boot {
    Uefi,
}

impl FromStr for Boot {
    type Err = String;
    fn from_str(x: &str) -> Result<Self, Self::Err> {
        match x {
            "uefi" => Ok(Boot::Uefi),
            _ => Err(format!("Unsupported boot option: {}", x)),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Arch {
    AArch64,
}

impl Arch {
    pub fn to_str(&self) -> &'static str {
        match self {
            Arch::AArch64 => "aarch64",
        }
    }
}

impl FromStr for Arch {
    type Err = String;
    fn from_str(x: &str) -> Result<Self, Self::Err> {
        match x {
            "aarch64" => Ok(Arch::AArch64),
            _ => Err(format!("Unsupported architecture: {}", x)),
        }
    }
}

#[derive(Parser, Clone)]
pub struct CargoFlags {
    /// Target architecture.
    #[clap(long, default_value = "aarch64")]
    pub arch: Arch,
    /// Features for the kernel crate.
    #[clap(long)]
    pub features: Option<String>,
    /// Do release build.
    #[clap(long)]
    pub release: bool,
}

impl CargoFlags {
    pub fn user_traget(&self) -> String {
        assert_eq!(self.arch, Arch::AArch64);
        let target_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("sophon")
            .join(format!("{}-sophon.json", self.arch.to_str()));
        target_path.to_str().unwrap().to_owned()
    }

    pub fn kernel_module_traget(&self) -> String {
        self.user_traget()
    }

    pub fn kernel_target(&self) -> String {
        self.user_traget()
    }

    pub fn uefi_target(&self) -> &'static str {
        assert_eq!(self.arch, Arch::AArch64);
        "aarch64-uefi.json"
    }
}

// pub fn copy_file(from: impl AsRef<Path>, to: impl AsRef<Path>) {
//     xshell::Shell::new().unwrap().copy_file(from, to).unwrap();
// }

// pub fn mkdir(path: impl AsRef<Path>) {
//     xshell::Shell::new().unwrap().create_dir(path).unwrap();
// }

fn append_cargo_args<'a>(
    mut cmd: Cmd<'a>,
    features: Option<String>,
    release: bool,
    target: Option<&str>,
) -> Cmd<'a> {
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

pub trait ShellExt {
    fn disassemble(&self, bin: impl AsRef<Path>, out: impl AsRef<Path>);
    fn build_package(
        &self,
        path: impl AsRef<Path>,
        features: Option<String>,
        release: bool,
        target: Option<&str>,
    );
    fn run_package(
        &self,
        path: impl AsRef<Path>,
        features: Option<String>,
        release: bool,
        target: Option<&str>,
        args: &[String],
    );
}

impl ShellExt for Shell {
    fn disassemble(&self, bin: impl AsRef<Path>, out: impl AsRef<Path>) {
        let dissam = cmd!(self, "llvm-objdump")
            .args([
                "--section-headers",
                "--all-headers",
                "--source",
                "-D",
                bin.as_ref().to_str().unwrap(),
            ])
            .ignore_stderr()
            .read()
            .unwrap();
        fs::write(out, dissam).unwrap();
    }
    fn build_package(
        &self,
        path: impl AsRef<Path>,
        features: Option<String>,
        release: bool,
        target: Option<&str>,
    ) {
        let path = path.as_ref();
        let _p = self.push_dir(path);
        let mut cmd = cmd!(self, "cargo build").quiet();
        cmd = append_cargo_args(cmd, features, release, target);
        eprintln!("$ cd {} && {}", path.to_str().unwrap(), cmd);
        cmd.run().unwrap();
    }
    fn run_package(
        &self,
        path: impl AsRef<Path>,
        features: Option<String>,
        release: bool,
        target: Option<&str>,
        args: &[String],
    ) {
        let path = path.as_ref();
        let _p = self.push_dir(path);
        let mut cmd = cmd!(self, "cargo run").quiet();
        cmd = append_cargo_args(cmd, features, release, target);
        if !args.is_empty() {
            cmd = cmd.arg("--").args(args);
        }
        eprintln!("$ cd {} && {}", path.to_str().unwrap(), cmd);
        cmd.run().unwrap();
    }
}

pub fn load_yaml(path: impl AsRef<Path>) -> Vec<Yaml> {
    YamlLoader::load_from_str(&fs::read_to_string(path).unwrap()).unwrap()
}
