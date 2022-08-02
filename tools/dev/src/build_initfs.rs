use crate::util::{self, Arch, CargoFlags, ShellExt};
use std::{error::Error, fs};
use vfs::ramfs::{self, RamFS};
use xshell::Shell;
use yaml_rust::Yaml;

#[derive(Parser)]
pub struct BuildInitFS {
    /// Output file.
    #[clap(long, default_value = "target/_boot/init.fs")]
    pub out: String,
    #[clap(flatten)]
    pub cargo: CargoFlags,
}

impl BuildInitFS {
    fn gen_file(
        &self,
        shell: &Shell,
        path: &str,
        entry: &Yaml,
        fs: &mut RamFS,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(cargo_module) = entry["+ cargo-build"].as_str() {
            shell.build_package(
                cargo_module,
                self.cargo.features.clone(),
                self.cargo.release,
                Some(&self.cargo.kernel_module_traget()),
            );
        }
        if let Some(from) = entry["+ copy"].as_str() {
            let file = fs::read(from).unwrap();
            fs.insert(path, ramfs::File::new(file));
        }
        if let Some(data) = entry["+ copy-str"].as_str() {
            fs.insert(path, ramfs::File::new(data.as_bytes().to_vec()));
        }
        Ok(())
    }

    fn gen_dir(
        &self,
        shell: &Shell,
        path: &str,
        entries: &Yaml,
        fs: &mut RamFS,
    ) -> Result<(), Box<dyn Error>> {
        for (name, entry) in entries
            .as_hash()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.as_str().unwrap(), v))
        {
            if name.ends_with("/") {
                self.gen_dir(
                    shell,
                    &format!("{}/{}", path, name.split_at(name.len() - 1).0),
                    entry,
                    fs,
                )?;
            } else {
                self.gen_file(shell, &format!("{}/{}", path, name), entry, fs)?;
            }
        }
        Ok(())
    }

    fn build_initfs(&self, shell: &Shell) {
        assert_eq!(self.cargo.arch, Arch::AArch64);
        // Create ram fs
        let mut init_fs = ramfs::RamFS::new();
        // Add files
        let docs = util::load_yaml("./Build.yml");
        let initfs = docs
            .iter()
            .find(|doc| !doc["init.fs"]["/"].is_badvalue())
            .unwrap();
        self.gen_dir(shell, "", &initfs["init.fs"]["/"], &mut init_fs)
            .unwrap();
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
