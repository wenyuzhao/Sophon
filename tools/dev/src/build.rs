use crate::{
    build_initfs::BuildInitFS,
    util::{self, CargoFlags},
};

#[derive(Clap)]
pub struct Build {
    #[clap(default_value = "uefi")]
    pub boot: String,
    #[clap(flatten)]
    pub cargo: CargoFlags,
}

impl Build {
    pub fn run(&self) {
        assert_eq!(self.boot, "uefi");
        assert_eq!(self.cargo.arch, "aarch64");
        // Build kernel
        util::build_package(
            "sophon",
            "sophon",
            self.cargo.features.clone(),
            self.cargo.release,
            Some(self.cargo.kernel_target()),
        );
        // Build init.fs
        let build_initfs = BuildInitFS {
            cargo: self.cargo.clone(),
            out: "./target/_boot/init.fs".to_string(),
        };
        build_initfs.run();
        // Build bootloader
        util::build_package(
            "sophon-boot-uefi",
            "boot/uefi",
            None,
            self.cargo.release,
            Some(self.cargo.uefi_target()),
        );
        // Build image
        util::mkdir("./target/_boot");
        util::mkdir("./target/_boot/EFI/BOOT");
        //  - copy kernel
        util::copy_file("./target/_out/sophon", "./target/_boot/");
        //  - copy efi bootloader.
        // FIXME: Use BOOTX64.EFI for x86_64.
        util::copy_file(
            "./target/_out/sophon-boot-uefi.efi",
            "./target/_boot/EFI/BOOT/BOOTAA64.EFI",
        );
    }
}
