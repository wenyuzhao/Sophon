use xshell::Shell;

use crate::{
    build::Build,
    util::{Arch, Boot, CargoFlags, ShellExt},
};

#[derive(Parser)]
pub struct Run {
    /// Boot option.
    #[clap(long, default_value = "uefi")]
    pub boot: Boot,
    #[clap(flatten)]
    pub cargo: CargoFlags,
    #[clap(last = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}

impl Run {
    pub fn run(&self, shell: &Shell) {
        assert_eq!(self.boot, Boot::Uefi);
        assert_eq!(self.cargo.arch, Arch::AArch64);
        // Build
        let build = Build {
            boot: self.boot,
            cargo: self.cargo.clone(),
        };
        build.run(shell, true);
        // Run
        let code = shell.run_package(
            "boot/uefi",
            self.cargo.features.clone(),
            self.cargo.release,
            Some(self.cargo.uefi_target()),
            &self.args,
        );
        std::process::exit(code);
    }
}
