use crate::gpio::PERIPHERAL_BASE;
use spin::Mutex;
use core::intrinsics::volatile_load;
use cortex_a::asm;

const VIDEOCORE_MAILBOX_BASE: usize = PERIPHERAL_BASE + 0xB880;



pub trait Request {
    const TAG_ID: u32;
    const TAG_VALUE_SIZE: usize;
    type Response;
}

pub mod req {
    use super::{*, misc::*};

    #[repr(C)] pub struct GetFirmwireRevision;
    #[repr(C)] pub struct GetARMMemory;
    #[repr(C)] pub struct GetVCMemory;
    #[repr(C)] pub struct SetPowerState { pub device: u32, pub state: u32 }
    #[repr(C)] pub struct SetClockRate { pub clock: Clock, pub rate: u32, /** 0 or 1 */pub skip_setting_turbo: u32 }
    #[repr(C)] pub struct AllocateBuffer { /** Alignment in bytes */ pub alignment: u32 }
    #[repr(C)] pub struct GetPhysicalResolution;
    #[repr(C)] pub struct GetVirtualResolution;
    #[repr(C)] pub struct GetPitch;
    #[repr(C)] pub struct SetPhysicalResolution { pub width: u32, pub height: u32 }
    #[repr(C)] pub struct SetVirtualResolution { pub width: u32, pub height: u32 }
    #[repr(C)] pub struct SetDepth(pub u32);
    #[repr(C)] pub struct SetPixelOrder(pub PixelOrder);
    #[repr(C)] pub struct SetAlphaMode(pub AlphaMode);
    #[repr(C)] pub struct SetVirtualOffset { pub x: u32, pub y: u32 }
}

pub mod res {
    use super::{*, misc::*};

    #[repr(C)] pub struct GetFirmwireRevision(pub u32);
    #[repr(C)] pub struct GetARMMemory { pub base_address: u32, pub size: u32 }
    #[repr(C)] pub struct GetVCMemory { pub base_address: u32, pub size: u32 }
    #[repr(C)] pub struct SetPowerState { pub device: u32, pub state: u32 }
    #[repr(C)] pub struct SetClockRate { pub clock: Clock, pub rate: u32 }
    #[repr(C)] pub struct AllocateBuffer { pub base_address: u32, pub size: u32 }
    #[repr(C)] pub struct GetPhysicalResolution { pub width: u32, pub height: u32 }
    #[repr(C)] pub struct GetVirtualResolution { pub width: u32, pub height: u32 }
    #[repr(C)] pub struct GetPitch(pub u32);
    #[repr(C)] pub struct SetPhysicalResolution { pub width: u32, pub height: u32 }
    #[repr(C)] pub struct SetVirtualResolution { pub width: u32, pub height: u32 }
    #[repr(C)] pub struct SetDepth(pub u32);
    #[repr(C)] pub struct SetPixelOrder(pub PixelOrder);
    #[repr(C)] pub struct SetAlphaMode(pub AlphaMode);
    #[repr(C)] pub struct SetVirtualOffset { pub x: u32, pub y: u32 }
}

macro_rules! register_tag {
    ($t:ident : tag = $tagid:literal, size = $tagsize:literal) => {
        impl Request for req::$t {
            const TAG_ID: u32 = $tagid;
            const TAG_VALUE_SIZE: usize = $tagsize;
            type Response = res::$t;
        }
    };
}

register_tag!(GetFirmwireRevision:   tag = 0x00000001, size = 4);
register_tag!(GetARMMemory:          tag = 0x00010005, size = 8);
register_tag!(GetVCMemory:           tag = 0x00010006, size = 8);
register_tag!(SetPowerState:         tag = 0x00028001, size = 8);
register_tag!(SetClockRate:          tag = 0x00038002, size = 12);
register_tag!(AllocateBuffer:        tag = 0x00040001, size = 8);
register_tag!(GetPhysicalResolution: tag = 0x00040003, size = 8);
register_tag!(GetVirtualResolution:  tag = 0x00040004, size = 8);
register_tag!(GetPitch:              tag = 0x00040008, size = 4);
register_tag!(SetPhysicalResolution: tag = 0x00048003, size = 8);
register_tag!(SetVirtualResolution:  tag = 0x00048004, size = 8);
register_tag!(SetDepth:              tag = 0x00048005, size = 4);
register_tag!(SetPixelOrder:         tag = 0x00048006, size = 4);
register_tag!(SetAlphaMode:          tag = 0x00048007, size = 4);
register_tag!(SetVirtualOffset:      tag = 0x00048009, size = 8);

pub struct MailBox;

impl MailBox {
    const MAILBOX_READ: *const u32 = (VIDEOCORE_MAILBOX_BASE + 0x0) as _;
    const MAILBOX_WRITE: *mut u32 = (VIDEOCORE_MAILBOX_BASE + 0x20) as _;
    const MAILBOX_RESPONSE_OK: u32 = 0x80000000;
    const MAILBOX_RESPONSE_ERR: u32 = 0x80000001;

    pub fn send<R: Request>(channel: Channel, request: R) -> Result<R::Response, MailBoxError> {
        debug_assert!(::core::mem::size_of::<R>() & 0b11 == 0);
        debug_assert!(::core::mem::size_of::<R::Response>() & 0b11 == 0);
        debug_assert!(R::TAG_VALUE_SIZE & 0b11 == 0);
        #[repr(C, align(16))] struct MBBuffer([u32; 16]);
        let mut buffer = MBBuffer([0u32; 16]);
        buffer.0[0] = 16 * 4;      // Buffer size
        buffer.0[1] = 0;           // Request code
        buffer.0[2] = R::TAG_ID;   // Tag identifier
        buffer.0[3] = R::TAG_VALUE_SIZE as u32; // Request value buffer size
        buffer.0[4] = R::TAG_VALUE_SIZE as u32; // Response value buffer size
        // Values
        let values_ptr = &mut buffer.0[5] as *mut _ as usize as *mut R;
        unsafe { ::core::ptr::write(values_ptr, request); }
        // End Tag
        buffer.0[5 + R::TAG_VALUE_SIZE >> 0b11] = 0;
        // Send buffer
        let buffer_address = &buffer as *const _ as usize;
        debug_assert!(buffer_address & 0xF == 0);
        debug_assert!(buffer_address == &buffer.0[0] as *const _ as usize);
        let message = (buffer_address & !0xF) as u32 | (channel as u8 & 0xF) as u32;
        while MailBoxStatus::get() == MailBoxStatus::Full {
            asm::nop();
        }
        unsafe { *Self::MAILBOX_WRITE = message; }
        loop {
            while MailBoxStatus::get() == MailBoxStatus::Empty {
                asm::nop();
            }
            if unsafe { *Self::MAILBOX_READ } == message {
                return match buffer.0[1] {
                    Self::MAILBOX_RESPONSE_OK => {
                        let ptr = &buffer.0[5] as *const _ as usize as *const R::Response;
                        Ok(unsafe { ::core::ptr::read(ptr) })
                    },
                    Self::MAILBOX_RESPONSE_ERR => Err(MailBoxError::ErrorParsingRequestBuffer(buffer.0[1])),
                    _ => Err(MailBoxError::Other(buffer.0[1])),
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MailBoxError {
    ErrorParsingRequestBuffer(u32),
    Other(u32)
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    PropertyARM2VC = 8, // VC: VideoCore
    PropertyVC2ARM = 9,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MailBoxStatus {
    Uninitialized = 0,
    Empty = 0x40000000,
    Full = 0x80000000,
}

impl MailBoxStatus {
    const ADDRESS: *mut MailBoxStatus = (VIDEOCORE_MAILBOX_BASE + 0x18) as _;
    
    #[inline]
    pub fn get() -> Self {
        unsafe { volatile_load(Self::ADDRESS) }
    }
}



// Helper structs for building requests / responses

pub mod misc {
    #[repr(u32)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum PixelOrder {
        BGR = 0x0,
        RGB = 0x1,
    }

    #[repr(u32)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum AlphaMode {
        Enabled = 0x0,  // 0 = fully opaque
        Reversed = 0x1, // 0 = fully transparent
        Ignored = 0x2,  // ignored
    }

    #[repr(u32)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Clock {
        _Reserved = 0x000000000,
        EMMC = 0x000000001,
        UART = 0x000000002,
        ARM = 0x000000003,
        CORE = 0x000000004,
        V3D = 0x000000005,
        H264 = 0x000000006,
        ISP = 0x000000007,
        SDRAM = 0x000000008,
        PIXEL = 0x000000009,
        PWM = 0x00000000a,
        EMMC2 = 0x00000000c,
    }
}