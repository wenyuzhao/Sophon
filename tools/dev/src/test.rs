use xshell::Shell;

use crate::{
    run::Run,
    util::{Arch, Boot, CargoFlags},
};

#[derive(Parser)]
pub struct Test {
    /// Boot option.
    #[clap(long, default_value = "uefi")]
    boot: Boot,
    #[clap(flatten)]
    pub cargo: CargoFlags,
    #[clap(last = true, allow_hyphen_values = true)]
    args: Vec<String>,
}

impl Test {
    pub fn run(&self, shell: &Shell) {
        assert_eq!(self.boot, Boot::Uefi);
        assert_eq!(self.cargo.arch, Arch::AArch64);
        // Run with cfg!(sophon_test)
        std::env::set_var("RUSTFLAGS", "--cfg sophon_test");
        let run = Run {
            boot: self.boot,
            cargo: self.cargo.clone(),
            args: self.args.clone(),
        };
        run.run(shell);
    }
}
