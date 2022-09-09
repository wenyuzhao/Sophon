use xshell::Shell;

use crate::{
    build_initfs::BuildInitFS,
    util::{Arch, Boot, CargoFlags, ShellExt},
};

#[derive(Parser)]
pub struct Build {
    /// Boot option.
    #[clap(long, default_value = "uefi")]
    pub boot: Boot,
    #[clap(flatten)]
    pub cargo: CargoFlags,
}

impl Build {
    pub fn run(&self, shell: &Shell, qemu: bool) {
        assert_eq!(self.boot, Boot::Uefi);
        assert_eq!(self.cargo.arch, Arch::AArch64);
        // Build kernel
        let features = if qemu {
            match self.cargo.features.clone() {
                Some(features) => Some(format!("{},qemu", features)),
                _ => Some("qemu".to_owned()),
            }
        } else {
            self.cargo.features.clone()
        };
        shell.build_package(
            "sophon",
            features,
            self.cargo.release,
            Some(&self.cargo.kernel_target()),
        );
        // Build init.fs
        let build_initfs = BuildInitFS {
            cargo: self.cargo.clone(),
            out: "./target/_boot/init.fs".to_string(),
        };
        build_initfs.run(shell);
        let rflags = std::env::var("RUSTFLAGS");
        assert!(
            rflags.is_err() || rflags == Ok("--cfg sophon_test".to_owned()),
            "RUSTFLAGS is set to {:?}",
            rflags
        );
        std::env::remove_var("RUSTFLAGS");
        // Build bootloader
        shell.build_package(
            "boot/uefi",
            None,
            self.cargo.release,
            Some(self.cargo.uefi_target()),
        );
        // Build image
        shell.create_dir("./target/_boot").unwrap();
        shell.create_dir("./target/_boot/EFI/BOOT").unwrap();
        //  - copy kernel
        shell
            .copy_file("./target/_out/sophon", "./target/_boot/")
            .unwrap();
        //  - copy efi bootloader.
        // FIXME: Use BOOTX64.EFI for x86_64.
        shell
            .copy_file(
                "./target/_out/sophon-boot-uefi.efi",
                "./target/_boot/EFI/BOOT/BOOTAA64.EFI",
            )
            .unwrap();
    }
}
