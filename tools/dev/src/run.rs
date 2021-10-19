use crate::{
    build::Build,
    util::{self, CargoFlags},
};

#[derive(Clap)]
pub struct Run {
    #[clap(default_value = "uefi")]
    boot: String,
    #[clap(flatten)]
    pub cargo: CargoFlags,
}

impl Run {
    pub fn run(&self) {
        assert_eq!(self.boot, "uefi");
        assert_eq!(self.cargo.arch, "aarch64");
        // Build
        let build = Build {
            boot: self.boot.clone(),
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
        );
    }
}
