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
    fn build_user(&self, shell: &Shell, name: &str) -> String {
        shell.build_package(
            &name,
            format!("user/{}", name),
            self.cargo.features.clone(),
            self.cargo.release,
            Some(&self.cargo.user_traget()),
        );
        format!("./target/_out/{}", name)
    }

    fn build_initfs(&self, shell: &Shell) {
        assert_eq!(self.cargo.arch, Arch::AArch64);
        // Create ram fs
        let mut init_fs = initfs::InitFS::default();
        // Add files
        let docs = util::load_yaml("./user/Build.yml");
        let config = &docs[0];
        let init_fs_files = config["init.fs"].as_hash().unwrap();
        for (name, path) in init_fs_files
            .iter()
            .map(|(k, v)| (k.as_str().unwrap(), v.as_str().unwrap()))
        {
            let out = self.build_user(shell, name);
            let file = fs::read(out).unwrap();
            init_fs.insert(path, initfs::File::new(file));
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
