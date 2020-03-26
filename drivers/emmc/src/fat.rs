
static mut MBR: [u8; 512] = [0; 512];
static mut PARTITION_LBA: u32 = 0;

pub struct FAT;

impl FAT {
    pub fn init() -> Result<(), ()> {
        unsafe {
            let bpb = &mut *(&mut MBR[0] as *mut u8 as usize as *mut BPB);
            super::emmc::EMMC::read_block(0, &mut MBR, 1).unwrap();
            assert!(MBR[510] == 0x55 && MBR[511] == 0xAA, "Bad magic: 0x{:02x}{:02x}", MBR[510], MBR[511]);
            // assert!(MBR[0x1C2] == 0xE || MBR[0x1C2] == 0xC, "Incorrect partition type: 0x{:x}", MBR[0x1C2]);
            log!("MBR disk identifier: 0x{:x}", *(&MBR[0x1B8] as *const u8 as usize as *const u32));
            PARTITION_LBA = *(&MBR[0x1C6] as *const u8 as usize as *const u32);
            log!("FAT partition starts at: 0x{:x}", PARTITION_LBA);
            super::emmc::EMMC::read_block(PARTITION_LBA, &mut MBR, 1).unwrap();
            assert! {
                (bpb.fst[0] == 'F' as u8 && bpb.fst[1] == 'A' as u8 && bpb.fst[2] == 'T' as u8)
             || (bpb.fst2[0] == 'F' as u8 && bpb.fst2[1] == 'A' as u8 && bpb.fst2[2] == 'T' as u8)
            };
            log!("FAT type: {}", if bpb.spf16 > 0 { "FAT16" } else { "FAT32" });
        }
        Ok(())
    }

    pub fn ls_root() {
        unsafe {
            let bpb = &mut *(&mut MBR[0] as *mut u8 as usize as *mut BPB);
            // unsigned int root_sec, s;
            // find the root directory's LBA
            let mut root_sec = {
                let spf = if bpb.spf16 != 0 { bpb.spf16 as u32 } else { bpb.spf32 };
                spf * bpb.nf as u32 + bpb.rsc as u32
            };
            // root_sec=((bpb->spf16?bpb->spf16:bpb->spf32)*bpb->nf)+bpb->rsc;
            //WARNING gcc generates bad code for bpb->nr, causing unaligned exception
            let mut s = (MBR[17] as u32 + ((MBR[18] as u32) << 8)) as u32;
            log!("FAT number of root diretory entries: 0x{:x}", s);
            s <<= 5;
            // now s=bpb->nr*sizeof(fatdir_t));
            if bpb.spf16 == 0 {
                // adjust for FAT32
                root_sec += (bpb.rc - 2) * bpb.spc as u32;
            }
            // add partition LBA
            root_sec += PARTITION_LBA;
            log!("FAT root directory LBA: 0x{:x}", root_sec);
            // load the root directory
            super::emmc::EMMC::read_block(root_sec, &mut MBR, s / 512 + 1).expect("Unable to read root directory");
            log!("Attrib Cluster  Size     Name");
            // iterate on each entry and print out
            let mut dir_ptr = &mut MBR[0] as *mut u8 as usize as *mut FATDir;
            while (*dir_ptr).name[0] != 0 {
                let dir = &mut *dir_ptr;
                // is it a valid entry?
                if dir.name[0] == 0xE5 || dir.attr[0] == 0xF {
                    dir_ptr = dir_ptr.offset(1);
                    continue;
                }
                // decode attributes
                let a = dir.attr[0];
                log!(noeol: "{}", if a &  1 != 0 { 'R' } else { '.' }); // read-only
                log!(noeol: "{}", if a &  2 != 0 { 'H' } else { '.' }); // hidden
                log!(noeol: "{}", if a &  4 != 0 { 'S' } else { '.' }); // system
                log!(noeol: "{}", if a &  8 != 0 { 'L' } else { '.' }); // volume label
                log!(noeol: "{}", if a & 16 != 0 { 'D' } else { '.' }); // directory
                log!(noeol: "{}", if a & 32 != 0 { 'A' } else { '.' }); // archive
                log!(noeol: " ");
                // staring cluster
                log!(noeol: "{:7x} ", (dir.ch as u32) << 16 | dir.cl as u32);
                // size
                log!(noeol: "{:5}     ", dir.size);
                // filename
                dir.attr[0] = 0;
                let name_buf = &dir.name[0] as *const u8;
                print_cstr(name_buf);
                log!(" !");

                dir_ptr = dir_ptr.offset(1);
            }
        }
    }
}

fn print_cstr(mut c: *const u8) {
    unsafe {
        while *c != 0u8 {
            log!(noeol: "{}", *c as char);
            c = c.offset(1);
        }
    }
}

#[repr(packed)]
struct BPB {
    jmp: [u8; 3],
    oem: [u8; 8],
    bps0: u8,
    bps1: u8,
    spc: u8,
    rsc: u16,
    nf: u8,
    nr0: u8,
    nr1: u8,
    ts16: u16,
    media: u8,
    spf16: u16,
    spt: u16,
    nh: u16,
    hs: u32,
    ts32: u32,
    spf32: u32,
    flg: u32,
    rc: u32,
    vol: [u8; 6],
    fst: [u8; 8],
    dmy: [u8; 20],
    fst2: [u8; 8],
}

#[repr(packed)]
struct FATDir {
    name: [u8; 8],
    ext: [u8; 3],
    attr: [u8; 9],
    ch: u16,
    attr2: u32,
    cl: u16,
    size: u32,
}