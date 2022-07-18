use crate::util::{self, Arch, CargoFlags, ShellExt};
use std::fs;
use xshell::Shell;

mod initfs {
    include!(concat!(env!("OUT_DIR"), "/init-fs.rs"));
}

#[derive(Parser)]
pub struct BuildInitFS {
    /// Output file.
    #[clap(long, default_value = "target/_boot/init.fs")]
    pub out: String,
    #[clap(flatten)]
    pub cargo: CargoFlags,
}

impl BuildInitFS {
    fn build_kernel_module(&self, shell: &Shell, name: &str) -> String {
        let (_, target_path) = self.cargo.kernel_module_traget();
        shell.build_package(
            &name,
            format!("modules/{}", name),
            self.cargo.features.clone(),
            self.cargo.release,
            Some(&target_path),
        );
        format!("./target/_out/{}", format!("lib{}.so", name))
    }

    fn build_user(&self, shell: &Shell, name: &str) -> String {
        let (_, target_path) = self.cargo.user_traget();
        shell.build_package(
            &name,
            format!("user/{}", name),
            self.cargo.features.clone(),
            self.cargo.release,
            Some(&target_path),
        );
        format!("./target/_out/{}", name)
    }

    fn build_initfs(&self, shell: &Shell) {
        assert_eq!(self.cargo.arch, Arch::AArch64);
        // Create ram fs
        let mut init_fs = initfs::InitFS::default();
        // Add files
        let docs = util::load_yaml("./Build.yml");
        let config = &docs[0];
        // Copy kernel modules
        if let Some(modules) = config["init.fs"]["modules"].as_hash() {
            for (name, path) in modules
                .iter()
                .map(|(k, v)| (k.as_str().unwrap(), v.as_str().unwrap()))
            {
                let out = self.build_kernel_module(shell, name);
                let file = fs::read(out).unwrap();
                init_fs.insert(path, initfs::File::new(file));
            }
        }
        // Copy user programs
        if let Some(programs) = config["init.fs"]["user"].as_hash() {
            for (name, path) in programs
                .iter()
                .map(|(k, v)| (k.as_str().unwrap(), v.as_str().unwrap()))
            {
                let out = self.build_user(shell, name);
                let file = fs::read(out).unwrap();
                init_fs.insert(path, initfs::File::new(file));
            }
        }
        // Serialize
        let data = init_fs.serialize();
        // Output
        shell.create_dir("./target/_boot").unwrap();
        fs::write(&self.out, data).unwrap();
    }

    pub fn run(&self, shell: &Shell) {
        assert_eq!(self.cargo.arch, Arch::AArch64);
        // Generate init.fs
        self.build_initfs(shell);
    }
}
