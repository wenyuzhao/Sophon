use proton_kernel::arch::AbstractBootImage;


static INIT_ELF: &'static [u8] = include_bytes!("../../../target/aarch64-unknown-none/debug/init");
static EMMC_ELF: &'static [u8] = include_bytes!("../../../target/aarch64-unknown-none/debug/emmc");

pub struct BootImage;

impl AbstractBootImage for BootImage {
    fn get(file: &str) -> Option<&'static [u8]> {
        match file {
            "init" => Some(INIT_ELF),
            "emmc" => Some(EMMC_ELF),
            _ => None,
        }
    }
}
