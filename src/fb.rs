use spin::Mutex;
use crate::mailbox::{Mail, Request, Response, PixelOrder, AlphaMode, Channel};

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
        let mut mail = Mail::new(Channel::PropertyARM2VC);
        
        mail.add(/*0*/ Request::GetPhysicalResolution)
            .add(/*1*/ Request::SetVirtualOffset { x: 0, y: 0 })
            .add(/*2*/ Request::SetDepth(32))
            .add(/*3*/ Request::SetPixelOrder(PixelOrder::RGB))
            .add(/*4*/ Request::SetAlphaMode(AlphaMode::Reversed))
            .add(/*5*/ Request::AllocateBuffer { alignment: 4096 })
            .add(/*6*/ Request::GetPitch);
       
        if let Ok(responese) = mail.send() {
            debug_assert!(responese[2] == Response::SetDepth(32));
            debug_assert!(responese[3] == Response::SetPixelOrder(PixelOrder::RGB));
            debug_assert!(responese[4] == Response::SetAlphaMode(AlphaMode::Reversed));
            match responese[0] {
                Response::GetPhysicalResolution { width, height } => {
                    self.width = width as _;
                    self.height = height as _;
                },
                _ => unreachable!(),
            }
            match responese[5] {
                Response::AllocateBuffer { base_address, .. } => {
                    self.fb = base_address as _;
                },
                _ => unreachable!(),
            }
            match responese[6] {
                Response::GetPitch(pitch) => {
                    self.pitch = pitch as _;
                },
                _ => unreachable!(),
            }
            debug!("Successfully initialize video output: {}x{} (rgba)", self.width, self.height);
        } else {
            debug!("Failed to initialize video output");
        }
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