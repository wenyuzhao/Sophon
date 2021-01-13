use proton_kernel::arch::AbstractBootImage;


#[cfg(debug_assertions)]
static INIT_ELF: &'static [u8] = include_bytes!("../../../target/aarch64-proton/debug/init");

#[cfg(not(debug_assertions))]
static INIT_ELF: &'static [u8] = include_bytes!("../../../target/aarch64-proton/release/init");
// static EMMC_ELF: &'static [u8] = include_bytes!("../../../target/aarch64-proton/emmc");

pub struct BootImage;

impl AbstractBootImage for BootImage {
    fn get(file: &str) -> Option<&'static [u8]> {
        None
    }
}
