use spin::Mutex;
use crate::mailbox::{*, misc::*};
use crate::mm::address::*;
use crate::mm::page::Size4K;

pub static FRAME_BUFFER: Mutex<FrameBuffer> = Mutex::new(FrameBuffer::new());

/// RGB Color
#[repr(C)]
#[derive(Eq, PartialEq, Copy, Clone)]
pub struct Color(u32);

impl Color {
    pub const BLACK: Self = Color::rgba(0x000000FF);
    pub const WHITE: Self = Color::rgba(0xFFFFFFFF);
    pub const RED:   Self = Color::rgba(0xFF0000FF);
    pub const GREEN: Self = Color::rgba(0x00FF00FF);
    pub const BLUE:  Self = Color::rgba(0x0000FFFF);

    pub const fn rgba(v: u32) -> Self {
        Self(u32::from_be(v))
    }

    pub const fn alpha(&self, v: u8) -> Self {
        Self((self.0 & 0x00FFFFFF) | ((v as u32) << 24))
    }
}

impl core::fmt::Debug for Color {
    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        write!(f, "Color({:x?})", u32::from_be(self.0))
    }
}

#[derive(Debug)]
pub struct FrameBuffer {
    pub width: usize,
    pub height: usize,
    pub pitch: usize, // Bytes per row in ther frame buffer
    fb: *mut u32,
}

unsafe impl Send for FrameBuffer {}
unsafe impl Sync for FrameBuffer {}

impl FrameBuffer {
    const fn new() -> Self {
        Self {
            width: 0,
            height: 0,
            pitch: 0,
            fb: 0usize as _,
        }
    }

    pub fn init(&mut self) {
        const CH: Channel = Channel::PropertyARM2VC;

        let res::GetFirmwireRevision(rev) = MailBox::send(CH, req::GetFirmwireRevision).unwrap();
        debug!("Revision = {:x}", rev);

        {
            let res::GetARMMemory { base_address, size } = MailBox::send(CH, req::GetARMMemory).unwrap();
            debug!("ARM Memory: base={:x} size={:x}", base_address, size);
            let res::GetVCMemory { base_address, size } = MailBox::send(CH, req::GetVCMemory).unwrap();
            debug!("VC Memory: base={:x} size={:x}", base_address, size);
        }

        let res::GetPhysicalResolution { width, height } = MailBox::send(CH, req::GetPhysicalResolution).unwrap();
        MailBox::send(CH, req::SetVirtualOffset { x: 0, y: 0 }).unwrap();
        MailBox::send(CH, req::SetDepth(32)).unwrap();
        MailBox::send(CH, req::SetPixelOrder(PixelOrder::RGB)).unwrap();
        MailBox::send(CH, req::SetAlphaMode(AlphaMode::Reversed)).unwrap();
        let res::AllocateBuffer { base_address, size } = MailBox::send(CH, req::AllocateBuffer { alignment: 4096 }).unwrap();
        let res::GetPitch(pitch) = MailBox::send(CH, req::GetPitch).unwrap();

        self.width = width as _;
        self.height = height as _;
        self.pitch = pitch as _;
        self.fb = base_address as usize as *mut _;

        debug!("Successfully initialize video output: {}x{} (rgba)", self.width, self.height);
        debug!("Frame buffer = {:?}", self.fb);
    }

    #[inline(always)]
    pub fn set(&self, row: usize, col: usize, color: Color) {
        unsafe { 
            let ptr = self.fb.offset((row * self.width + col) as isize);
            *ptr = color.0;
        }
    }

    pub fn clear(&self, color: Color) {
        for i in 0..self.height {
            for j in 0..self.width {
                self.set(i, j, color);
            }
        }
    }
}