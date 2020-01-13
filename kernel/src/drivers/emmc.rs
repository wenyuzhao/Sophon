use crate::arch::aarch64::constants::*;
use crate::arch::*;

static mut HV: u32 = 0;
static mut RCA: u32 = 0;
static mut SCR: [u32; 2] = [0; 2];


pub type E = ();

pub struct EMMC;

impl EMMC {
    unsafe fn wait_for(mask: u32) {
        let emmc = &mut *emmc::BASE;
        while (emmc.status & mask) != 0 && (emmc.interrupt & emmc::INT_ERROR_MASK) == 0 {}
        // println!("EMMC_INTERRUPT {:?} {:?}", *EMMC_INTERRUPT, *EMMC_INTERRUPT & INT_ERROR_MASK);
        if emmc.interrupt & emmc::INT_ERROR_MASK != 0 {
            panic!("EMMC INT ERROR");
        }
        // if 0 != (*EMMC_INTERRUPT & INT_ERROR_MASK) { false } else { true }
    }

    unsafe fn int(mask: u32) {
        let emmc = &mut *emmc::BASE;
        let m = mask | emmc::INT_ERROR_MASK;
        while emmc.interrupt & m == 0 {}
        let r = emmc.interrupt;
        if (r & emmc::INT_CMD_TIMEOUT) != 0 || (r & emmc::INT_DATA_TIMEOUT) != 0 || (r & emmc::INT_ERROR_MASK) != 0 {
            emmc.interrupt = r;
            panic!();
        }
        emmc.interrupt = mask;
    }

    unsafe fn cmd(mut code: u32, arg: u32) -> Result<u32, E> {
        let emmc = &mut *emmc::BASE;
        if code & cmd::NEED_APP != 0 {
            let r = Self::cmd(cmd::APP_CMD | if RCA != 0 { cmd::RSPNS_48 } else { 0 }, RCA).unwrap();
            if RCA != 0 && r == 0 {
                panic!("ERROR: failed to send SD APP command");
            }
            code &= !cmd::NEED_APP;
        };
        Self::wait_for(emmc::SR_CMD_INHIBIT);
        println!("EMMC: Sending command {:x} arg {:x}", code, arg);
        emmc.interrupt = emmc.interrupt;
        emmc.arg1 = arg;
        emmc.cmdtm = code;
        if code == cmd::SEND_OP_COND {
            Target::Timer::wait(1000);
        } else {
            if code == cmd::SEND_IF_COND || code == cmd::APP_CMD {
                Target::Timer::wait(100);
            }
        }
        Self::int(emmc::INT_CMD_DONE);
        let mut r = emmc.resp[0];
        match code {
            x if x == cmd::GO_IDLE || x == cmd::APP_CMD => Ok(0),
            x if x == cmd::APP_CMD | cmd::RSPNS_48 => Ok(r & emmc::SR_APP_CMD),
            x if x == cmd::SEND_OP_COND => Ok(r),
            x if x == cmd::SEND_IF_COND => if r == arg { Ok(0) } else { Err(()) },
            x if x == cmd::ALL_SEND_CID => Ok(r | emmc.resp[3] | emmc.resp[2] | emmc.resp[1]),
            x if x == cmd::SEND_REL_ADDR => {
                let err = (((r & 0x1fff)) | ((r & 0x2000) << 6) | ((r & 0x4000) << 8) | ((r & 0x8000) << 8)) & cmd::ERRORS_MASK;
                if err == 0 { Ok(r & cmd::RCA_MASK) } else { Err(()) }
            }
            _ => Ok(r & cmd::ERRORS_MASK)
        }
    }

    unsafe fn set_clk(f: u32) {
        let emmc = &mut *emmc::BASE;
        let mut d: u32 = 0;
        let mut c = 41666666 / f;
        let mut x: u32;
        let mut s = 32u32;
        let mut h = 0u32;
        // int cnt = 100000;
        while (emmc.status & (emmc::SR_CMD_INHIBIT | emmc::SR_DAT_INHIBIT)) != 0 {
            Target::Timer::wait(1);
        }
    
        emmc.control1 &= !emmc::C1_CLK_EN;
        Target::Timer::wait(10);
        // wait_cycles(1000);
        x = c - 1;
        if x == 0 {
            s = 0;
        } else {
            if x & 0xffff0000 == 0 { x <<= 16; s -= 16; }
            if x & 0xff000000 == 0 { x <<= 8;  s -= 8; }
            if x & 0xf0000000 == 0 { x <<= 4;  s -= 4; }
            if x & 0xc0000000 == 0 { x <<= 2;  s -= 2; }
            if x & 0x80000000 == 0 { x <<= 1;  s -= 1; }
            if s > 0 { s -= 1; }
            if s > 7 { s=7; }
        }
        if HV > emmc::HOST_SPEC_V2 {
            d = c;
        } else {
            d = 1 << s;
        }
        if d<=2 {
            d = 2;
            s = 0;
        }
        println!("sd_clk divisor {:x}, shift {:x}", d, s);
        if HV > emmc::HOST_SPEC_V2 {
            h = (d & 0x300) >> 2;
        }
        d= ((d & 0xff) << 8) | h;
        emmc.control1 = (emmc.control1 & 0xffff003f) | d;
        Target::Timer::wait(10);
        // wait_msec(10);
        // wait_cycles(1000);
        emmc.control1 |= emmc::C1_CLK_EN;
        Target::Timer::wait(10);
        // wait_msec(10);
        // wait_cycles(1000);
        // cnt=10000; while(!(*EMMC_CONTROL1 & C1_CLK_STABLE) && cnt--) wait_msec(10);
        while emmc.control1 & emmc::C1_CLK_STABLE == 0 {
            Target::Timer::wait(10);
        }
        // if(cnt<=0) {
        //     panic!("ERROR: failed to get stable clock\n");
        // }
    }

    pub fn read_block(lba: u32, buffer: &mut [u8], mut num: u32) -> Result<u32, E> {
        unsafe {
            let emmc = &mut *emmc::BASE;
            if num == 0 { num = 1 }
            Self::wait_for(emmc::SR_DAT_INHIBIT);
            if SCR[0] & emmc::SCR_SUPP_CCS != 0 {
                if num > 1 && SCR[0] & emmc::SCR_SUPP_SET_BLKCNT != 0 {
                    Self::cmd(cmd::SET_BLOCKCNT, num).unwrap();
                }
                emmc.blksizecnt = (num << 16) | 512;
                Self::cmd(if num == 1 { cmd::READ_SINGLE } else { cmd::READ_MULTI }, lba).unwrap();
            } else {
                emmc.blksizecnt = (1 << 16) | 512;
            }
            let mut cursor = 0;
            for c in 0..num {
                if SCR[0] & emmc::SCR_SUPP_CCS == 0 {
                    Self::cmd(cmd::READ_SINGLE, (lba + c) * 512).unwrap();
                }
                Self::int(emmc::INT_READ_RDY);
                for i in 0..128 {
                    let u8_slot: &mut u8 = &mut buffer[cursor + i << 2];
                    let u32_slot = u8_slot as *mut u8 as usize as *mut u32;
                    *u32_slot = emmc.data;
                }
                cursor += 128;
            }
            println!("cursor = {}", cursor);
            if num > 1 && SCR[0] & emmc::SCR_SUPP_SET_BLKCNT == 0 && SCR[0] & emmc::SCR_SUPP_CCS != 0 {
                Self::cmd(cmd::STOP_TRANS, 0).unwrap();
            }
            Ok(num * 512)
        }
    }

    pub fn init() -> Result<(), ()> {
        let mut ccs = 0u32;
        unsafe {
            let emmc = &mut *emmc::BASE;
            // GPIO_CD
            *GPFSEL4 &= !(7 << (7 * 3));
            *GPPUD=2;
            wait_cycles(150);
            *GPPUDCLK1 = 1 << 15;
            wait_cycles(150);
            *GPPUD = 0;
            *GPPUDCLK1 = 0;
            *GPHEN1 |= 1 << 15;
            // GPIO_CLK, GPIO_CMD
            *GPFSEL4 |= (7 << (8 * 3)) | (7 << (9 * 3));
            *GPPUD=2;
            wait_cycles(150);
            *GPPUDCLK1= (1 << 16) | (1 << 17);
            wait_cycles(150);
            *GPPUD = 0;
            *GPPUDCLK1 = 0;
            // GPIO_DAT0, GPIO_DAT1, GPIO_DAT2, GPIO_DAT3
            *GPFSEL5 |= (7 << (0 * 3)) | (7 << (1 * 3)) | (7 << (2 * 3)) | (7 << (3 * 3));
            *GPPUD = 2;
            wait_cycles(150);
            *GPPUDCLK1 = (1<<18) | (1<<19) | (1<<20) | (1<<21);
            wait_cycles(150);
            *GPPUD = 0;
            *GPPUDCLK1 = 0;
            HV = (emmc.slotisr_ver & emmc::HOST_SPEC_NUM) >> emmc::HOST_SPEC_NUM_SHIFT;
            println!("EMMC: GPIO set up");


            emmc.control0 = 0;
            emmc.control1 |= emmc::C1_SRST_HC;
            while emmc.control1 & emmc::C1_SRST_HC != 0 {
                Target::Timer::wait(10);
            }
            
            println!("EMMC: reset OK");

            emmc.control1 |= emmc::C1_CLK_INTLEN | emmc::C1_TOUNIT_MAX;
            Target::Timer::wait(10);
            Self::set_clk(400000);
            emmc.int_en = 0xffffffff;
            emmc.int_mask = 0xffffffff;
            SCR = [0; 2];
            RCA = 0;
            Self::cmd(cmd::GO_IDLE, 0).unwrap();

            Self::cmd(cmd::SEND_IF_COND, 0x1AA).unwrap();
            {
                let mut r = 0;
                while (r & emmc::ACMD41_CMD_COMPLETE) == 0 {
                    wait_cycles(400);
                    r = Self::cmd(cmd::SEND_OP_COND, emmc::ACMD41_ARG_HC).unwrap();
                    print!("EMMC: CMD_SEND_OP_COND returned ");
                    if (r & emmc::ACMD41_CMD_COMPLETE) != 0 { print!("COMPLETE ") }
                    if (r & emmc::ACMD41_VOLTAGE) != 0 { print!("VOLTAGE ") }
                    if (r & emmc::ACMD41_CMD_CCS) != 0 { print!("CCS ") }
                    println!("0x{:x} 0x{:x}", r, r);
                }
                assert!((r & emmc::ACMD41_CMD_COMPLETE) != 0);
                assert!((r & emmc::ACMD41_VOLTAGE) != 0);
                if (r & emmc::ACMD41_CMD_CCS) != 0 {
                    ccs = emmc::SCR_SUPP_CCS;
                }
            }
            Self::cmd(cmd::ALL_SEND_CID, 0).unwrap();
            RCA = Self::cmd(cmd::SEND_REL_ADDR, 0).unwrap();
            println!("EMMC: CMD_SEND_REL_ADDR returned 0x{:x}", RCA);
            Self::set_clk(25000000);
            Self::cmd(cmd::CARD_SELECT, RCA).unwrap();
            Self::wait_for(emmc::SR_DAT_INHIBIT);
            emmc.blksizecnt = (1 << 16) | 8;
            Self::cmd(cmd::SEND_SCR, 0).unwrap();
            Self::int(emmc::INT_READ_RDY);
            {
                let mut i = 0;
                while i < 2 {
                    if (emmc.status & emmc::SR_READ_AVAILABLE) != 0 {
                        SCR[i] = emmc.data;
                        i += 1;
                    } else {
                        Target::Timer::wait(1);
                    }
                }
            }
            if SCR[0] & emmc::SCR_SD_BUS_WIDTH_4 != 0 {
                Self::cmd(cmd::SET_BUS_WIDTH,RCA | 2).unwrap();
                emmc.control0 |= emmc::C0_HCTL_DWITDH;
            }
            println!("0x{:x}", SCR[0]);
            print!("EMMC: supports ");
            if SCR[0] & emmc::SCR_SUPP_SET_BLKCNT != 0 { print!("SET_BLKCNT") }
            if ccs != 0 { print!(" CCS") }
            println!("");
            SCR[0] &= !emmc::SCR_SUPP_CCS;
            SCR[0] |= ccs;
        }
        Ok(())
    }
}

#[cfg(feature="device-raspi3-qemu")]
pub const MMIO_BASE: usize = 0xFFFF0000_3F000000;
#[cfg(feature="device-raspi4")]
pub const MMIO_BASE: usize = 0xFFFF0000_FE000000;

#[repr(C)]
struct EMMCData {
    arg2: u32,
    blksizecnt: u32,
    arg1: u32,
    cmdtm: u32,
    resp: [u32; 4],
    data: u32,
    status: u32,
    control0: u32,
    control1: u32,
    interrupt: u32,
    int_mask: u32,
    int_en: u32,
    control2: u32, // 0x30003C
    _0: [u8; 0x3000FC - 0x30003C - 4],
    slotisr_ver: u32, // 0x3000FC
}

mod emmc {
    pub const BASE: *mut super::EMMCData = (super::MMIO_BASE + 0x300000) as _;
    pub const SR_READ_AVAILABLE: u32 =   0x00000800;
    pub const SR_DAT_INHIBIT: u32 =      0x00000002;
    pub const SR_CMD_INHIBIT: u32 =      0x00000001;
    pub const SR_APP_CMD: u32 =          0x00000020;
    pub const INT_DATA_TIMEOUT: u32 =    0x00100000;
    pub const INT_CMD_TIMEOUT: u32 =     0x00010000;
    pub const INT_READ_RDY: u32 =        0x00000020;
    pub const INT_CMD_DONE: u32 =        0x00000001;
    pub const INT_ERROR_MASK: u32 =      0x017E8000;
    pub const C0_SPI_MODE_EN: u32 =      0x00100000;
    pub const C0_HCTL_HS_EN: u32 =       0x00000004;
    pub const C0_HCTL_DWITDH: u32 =      0x00000002;
    pub const C1_SRST_DATA: u32 =        0x04000000;
    pub const C1_SRST_CMD: u32 =         0x02000000;
    pub const C1_SRST_HC: u32 =          0x01000000;
    pub const C1_TOUNIT_DIS: u32 =       0x000f0000;
    pub const C1_TOUNIT_MAX: u32 =       0x000e0000;
    pub const C1_CLK_GENSEL: u32 =       0x00000020;
    pub const C1_CLK_EN: u32 =           0x00000004;
    pub const C1_CLK_STABLE: u32 =       0x00000002;
    pub const C1_CLK_INTLEN: u32 =       0x00000001;
    pub const HOST_SPEC_NUM: u32 =       0x00ff0000;
    pub const HOST_SPEC_NUM_SHIFT: u32 = 16;
    pub const HOST_SPEC_V3: u32 =        2;
    pub const HOST_SPEC_V2: u32 =        1;
    pub const HOST_SPEC_V1: u32 =        0;
    pub const SCR_SD_BUS_WIDTH_4: u32 =  0x00000400;
    pub const SCR_SUPP_SET_BLKCNT: u32 = 0x02000000;
    pub const SCR_SUPP_CCS: u32 =        0x00000001;
    pub const ACMD41_VOLTAGE: u32 =      0x00ff8000;
    pub const ACMD41_CMD_COMPLETE: u32 = 0x80000000;
    pub const ACMD41_CMD_CCS: u32 =      0x40000000;
    pub const ACMD41_ARG_HC: u32 =       0x51ff8000;
} 

mod cmd {
    pub static NEED_APP: u32      =        0x80000000;
    pub static RSPNS_48: u32      =        0x00020000;
    pub static ERRORS_MASK: u32 =     0xfff9c004;
    pub static RCA_MASK: u32 =        0xffff0000;
    pub static GO_IDLE: u32 =         0x00000000;
    pub static ALL_SEND_CID: u32 =    0x02010000;
    pub static SEND_REL_ADDR: u32 =   0x03020000;
    pub static CARD_SELECT: u32 =     0x07030000;
    pub static SEND_IF_COND: u32 =    0x08020000;
    pub static STOP_TRANS: u32 =      0x0C030000;
    pub static READ_SINGLE: u32 =     0x11220010;
    pub static READ_MULTI: u32 =      0x12220032;
    pub static SET_BLOCKCNT: u32 =    0x17020000;
    pub static APP_CMD: u32 =         0x37000000;
    pub static SET_BUS_WIDTH: u32 =   0x06020000 | NEED_APP;
    pub static SEND_OP_COND: u32 =    0x29020000 | NEED_APP;
    pub static SEND_SCR: u32 =        0x33220010 | NEED_APP;
}

fn wait_cycles(n: usize) {
    for _ in 0..n {}
}