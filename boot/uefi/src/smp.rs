use boot::BootInfo;
use cortex_a::registers::*;
use memory::address::Address;
use spin::{Barrier, Mutex, RwLock};
use tock_registers::interfaces::{Readable, Writeable};

#[derive(Clone)]
pub struct APBootInfo {
    entry: extern "C" fn(&mut BootInfo, core: usize) -> !,
    p4: Address,
}

impl APBootInfo {
    pub fn new(entry: extern "C" fn(&mut BootInfo, core: usize) -> !, p4: Address) -> Self {
        Self { entry, p4 }
    }
}

static AP_BOOT_INFO: Mutex<Option<APBootInfo>> = Mutex::new(None);
pub static BARRIER: RwLock<Option<Barrier>> = RwLock::new(None);
static SYNC: Mutex<()> = Mutex::new(());

pub fn boot_and_prepare_ap(
    num_cpus: usize,
    p4: Address,
    entry: extern "C" fn(&mut BootInfo, core: usize) -> !,
) {
    *BARRIER.write() = Some(Barrier::new(num_cpus));
    *AP_BOOT_INFO.lock() = Some(APBootInfo::new(entry, p4));
    for i in 1..num_cpus {
        let stack_top = super::new_page4k().end();
        let _ = psci::cpu_on(i as _, ap_entry_raw as _, stack_top.as_usize() as _);
    }
    BARRIER.read().as_ref().unwrap().wait();
}

fn current_core_id(way: usize) -> usize {
    let v = MPIDR_EL1.get() as usize;
    ((v >> 8) & 0xf) * way + (v & 0xff)
}

pub extern "C" fn start_ap() {
    BARRIER.read().as_ref().unwrap().wait();
}

#[no_mangle]
unsafe extern "C" fn ap_entry() {
    BARRIER.read().as_ref().unwrap().wait();
    let core = current_core_id(1);
    {
        let _guard = SYNC.lock();
        log!("Hello, AP #{}!", core);
    }
    let boot = AP_BOOT_INFO.lock().clone().unwrap();
    TTBR0_EL1.set(boot.p4.as_usize() as u64);
    BARRIER.read().as_ref().unwrap().wait();
    start_core(core, boot.entry, &mut crate::BOOT_INFO)
}

#[cfg(target_arch = "x86_64")]
pub extern "C" fn start_core(
    _id: usize,
    _entry: extern "C" fn(&mut BootInfo, usize) -> !,
    _boot_info: &'static mut BootInfo,
) -> ! {
    unimplemented!()
}

#[cfg(target_arch = "aarch64")]
pub extern "C" fn start_core(
    id: usize,
    entry: extern "C" fn(&mut BootInfo, usize) -> !,
    boot_info: &'static mut BootInfo,
) -> ! {
    use tock_registers::interfaces::*;
    CNTHCTL_EL2.write(CNTHCTL_EL2::EL1PCEN::SET + CNTHCTL_EL2::EL1PCTEN::SET);
    CNTVOFF_EL2.set(0);
    HCR_EL2.write(HCR_EL2::RW::EL1IsAarch64);
    MAIR_EL1.write(
        // Attribute 1 - Cacheable normal DRAM.
        MAIR_EL1::Attr1_Normal_Outer::WriteBack_NonTransient_ReadWriteAlloc +
        MAIR_EL1::Attr1_Normal_Inner::WriteBack_NonTransient_ReadWriteAlloc +
        // Attribute 0 - Device.
        MAIR_EL1::Attr0_Device::nonGathering_nonReordering_EarlyWriteAck,
    );
    TCR_EL1.write(
        //   TCR_EL1::IPS.val(0b101)
        TCR_EL1::TG0::KiB_4
            + TCR_EL1::TG1::KiB_4
            + TCR_EL1::SH0::Inner
            + TCR_EL1::SH1::Inner
            + TCR_EL1::ORGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable
            + TCR_EL1::IRGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable
            + TCR_EL1::ORGN1::WriteBack_ReadAlloc_WriteAlloc_Cacheable
            + TCR_EL1::IRGN1::WriteBack_ReadAlloc_WriteAlloc_Cacheable
            + TCR_EL1::EPD0::EnableTTBR0Walks
            + TCR_EL1::EPD1::EnableTTBR1Walks, // + TCR_EL1::T0SZ.val(0x10)
                                               // + TCR_EL1::T1SZ.val(0x10)
    );
    TCR_EL1.set(TCR_EL1.get() | 0b101 << 32); // Intermediate Physical Address Size (IPS) = 0b101
    TCR_EL1.set(TCR_EL1.get() | 0x10 << 0); // TTBR0_EL1 memory size (T0SZ) = 0x10 ==> 2^(64 - T0SZ)
    TCR_EL1.set(TCR_EL1.get() | 0x10 << 16); // TTBR1_EL1 memory size (T1SZ) = 0x10 ==> 2^(64 - T1SZ)

    SCTLR_EL1.set((3 << 28) | (3 << 22) | (1 << 20) | (1 << 11)); // Disable MMU
    SCTLR_EL1.modify(SCTLR_EL1::M::Enable + SCTLR_EL1::C::Cacheable + SCTLR_EL1::I::Cacheable);
    SPSR_EL2.write(
        SPSR_EL2::D::Masked
            + SPSR_EL2::A::Masked
            + SPSR_EL2::I::Masked
            + SPSR_EL2::F::Masked
            + SPSR_EL2::M::EL1h,
    );
    unsafe {
        core::arch::asm! {
            "
                mov x0, #0xfffffff
                msr cpacr_el1, x0
                mov x0, sp
                msr sp_el1, x0
            ",
            in("x0") 0,
            in("x1") 0,
        }
        ELR_EL2.set(crate::kernel_entry as *const () as u64);
        core::arch::asm! {
            "eret",
            in("x0") entry,
            in("x1") boot_info,
            in("x2") id,
        }
        unreachable!()
    }
}

extern "C" {
    fn ap_entry_raw(ctx: *mut core::ffi::c_void);
}

core::arch::global_asm! {"
.global ap_entry_raw

ap_entry_raw:
    mov sp, x0
    bl ap_entry
    b .
"}
