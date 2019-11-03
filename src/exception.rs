
#[no_mangle]
#[naked]
pub extern "C" fn exception_handler(kind: usize, esr: usize, elr: usize, spsr: usize, far: usize) -> ! {
    debug!("Exception {}: ESR={:x} ELR={:x} SPSR={:x} FAR={:x}", kind, esr, elr, spsr, far);
    unimplemented!();
}
