use cortex_a::regs::*;

#[repr(C, align(4096))]
struct PTT([usize; 512], [usize; 512], [usize; 512], [usize; 512]);

static mut PT0: PTT = PTT([0; 512], [0; 512], [0; 512], [0; 512]);
static mut PT1: PTT = PTT([0; 512], [0; 512], [0; 512], [0; 512]);

fn get_index(a: usize, level: usize) -> usize {
    let shift = (level - 1) * 9 + 12;
    (a >> shift) & 0b111111111
}

unsafe fn create_id_map() {
    // Identity map 0x0 ~ 0x200000
    let p4 = &PT0.0 as *const _ as usize as *mut [usize; 512];
    assert!(p4 as usize & 0xffff_0000_0000_0000 == 0);
    let p3 = &PT0.1 as *const _ as usize as *mut [usize; 512];
    let p2 = &PT0.2 as *const _ as usize as *mut [usize; 512];
    // Map p3 to p4
    (*p4)[get_index(p3 as _, 4)] = (p3 as usize | 0x3 | (1 << 10));
    // Map p2 tp p3
    (*p3)[get_index(p2 as _, 3)] = (p2 as usize | 0x3 | (1 << 10));
    // Map first block to p2
    (*p2)[0] = (0usize | 0x1 | (1 << 10));
    // Set page table register 0
    TTBR0_EL1.set(p4 as _);
}

pub unsafe fn setup_kernel_pagetables() {
    create_id_map();
    // Identity map 1GB of RAM: 0x0 ~ 0x40000000
    let p4 = &PT0.0 as *const _ as usize as *mut [usize; 512];
    assert!(p4 as usize & 0xffff_0000_0000_0000 == 0);
    let p3 = &PT0.1 as *const _ as usize as *mut [usize; 512];
    let p2 = &PT0.2 as *const _ as usize as *mut [usize; 512];
    let p1 = &PT0.3 as *const _ as usize as *mut [usize; 512];
    // Map p3 to p4
    (*p4)[get_index(p3 as _, 4)] = (p3 as usize | 0x3 | (1 << 10));
    // Map p2 tp p3
    (*p3)[get_index(p2 as _, 3)] = (p2 as usize | 0x3 | (1 << 10));
    // Map blocks
    {
        let mut block = 0x0;
        while block < 0x40000000 {
            (*p2)[get_index(block, 2)] = (block | 0x1 | (1 << 10) | (0b11 << 8));
            block += 0x200000;
        }
    }
    // Set page table
    TTBR1_EL1.set(p4 as _);
    // Set some extra MMU attributes
    const T0SZ: u64 = 0x10 << 0;
    const T1SZ: u64 = 0x10 << 16;
    TCR_EL1.set(T0SZ | T1SZ);
    #[allow(non_upper_case_globals)]
    const MT_DEVICE_nGnRnE: usize = 0;
    #[allow(non_upper_case_globals)]
    const MT_DEVICE_nGnRE: usize = 1;
    const MT_DEVICE_GRE:   usize = 2;
    const MT_NORMAL_NC:    usize = 3;
    const MT_NORMAL:       usize = 4;
    MAIR_EL1.set(
        (0x00u64 << (MT_DEVICE_nGnRnE * 8)) |
        (0x04u64 << (MT_DEVICE_nGnRE * 8)) |
        (0x0cu64 << (MT_DEVICE_GRE * 8)) |
        (0x44u64 << (MT_NORMAL_NC * 8)) |
        (0xffu64 << (MT_NORMAL * 8))
    );
    // Enable MMU
    SCTLR_EL1.set(SCTLR_EL1.get() | 0x1);
}
