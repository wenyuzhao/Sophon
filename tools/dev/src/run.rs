use crate::{
    build::Build,
    util::{self, Arch, Boot, CargoFlags},
};

#[derive(Clap)]
pub struct Run {
    /// Boot option.
    #[clap(long, default_value = "uefi")]
    boot: Boot,
    #[clap(flatten)]
    pub cargo: CargoFlags,
    #[clap(multiple = true)]
    args: Vec<String>,
}

impl Run {
    pub fn run(&self) {
        assert_eq!(self.boot, Boot::Uefi);
        assert_eq!(self.cargo.arch, Arch::AArch64);
        // Build
        let build = Build {
            boot: self.boot,
            cargo: self.cargo.clone(),
        };
        build.run();
        // Run
        util::run_package(
            "sophon-boot-uefi",
            "boot/uefi",
            self.cargo.features.clone(),
            self.cargo.release,
            Some(self.cargo.uefi_target()),
            &self.args,
        );
    }
}
