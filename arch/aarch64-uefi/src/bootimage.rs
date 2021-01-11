use proton_kernel::arch::AbstractBootImage;


static INIT_ELF: &'static [u8] = include_bytes!("../../../target/aarch64-proton/init");
// static EMMC_ELF: &'static [u8] = include_bytes!("../../../target/aarch64-proton/emmc");

pub struct BootImage;

impl AbstractBootImage for BootImage {
    fn get(file: &str) -> Option<&'static [u8]> {
        None
    }
}
