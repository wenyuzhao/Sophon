use crate::gpio::PERIPHERAL_BASE;
use spin::Mutex;
use core::intrinsics::volatile_load;

const VIDEOCORE_MAILBOX_BASE: usize = PERIPHERAL_BASE + 0xB880;

const BUFFER_ENTRIES: usize = 48;

#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MailBoxError {
    ErrorParsingRequestBuffer(u32),
    Other(u32)
}

#[allow(unused)]
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

static MAILBOX_BUFFER: Mutex<MailBoxBuffer> = Mutex::new(MailBoxBuffer([0; BUFFER_ENTRIES]));

#[repr(C, align(16))]
struct MailBoxBuffer([u32; BUFFER_ENTRIES]);

impl MailBoxBuffer {
    const MAILBOX_READ: *const u32 = (VIDEOCORE_MAILBOX_BASE + 0x0) as _;
    const MAILBOX_WRITE: *mut u32 = (VIDEOCORE_MAILBOX_BASE + 0x20) as _;
    const MAILBOX_RESPONSE_OK: u32 = 0x80000000;
    const MAILBOX_RESPONSE_ERR: u32 = 0x80000001;

    pub fn send(&mut self, ch: u8) -> Result<(), MailBoxError> {
        let message = ((self as *const _ as usize) & !0xF) as u32 | (ch & 0xF) as u32;
        while MailBoxStatus::get() == MailBoxStatus::Full {
            unsafe { asm!("nop"); }
        }
        unsafe {
            *Self::MAILBOX_WRITE = message;
        }
        loop {
            while MailBoxStatus::get() == MailBoxStatus::Empty {
                unsafe { asm!("nop"); }
            }
            if unsafe { volatile_load(Self::MAILBOX_READ) } == message {
                return match self.0[1] {
                    Self::MAILBOX_RESPONSE_OK => Ok(()),
                    Self::MAILBOX_RESPONSE_ERR => Err(MailBoxError::ErrorParsingRequestBuffer(self.0[1])),
                    _ => Err(MailBoxError::Other(self.0[1])),
                }
            }
        }
    }
}

#[allow(unused)]
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    PropertyARM2VC = 8, // VC: VideoCore
    PropertyVC2ARM = 9,
}

#[allow(unused)]
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelOrder {
    BGR = 0x0,
    RGB = 0x1,
}

#[allow(unused)]
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlphaMode {
    Enabled = 0x0, // 0 = fully opaque
    Reversed = 0x1, // 0 = fully transparent
    Ignored = 0x2, // ignored
}

#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Request {
    AllocateBuffer { /** Alignment in bytes */ alignment: u32 }, // 40001,
    GetPitch, // 0x40008
    SetPhysicalResolution { width: u32, height: u32 }, // 0x48003
    SetVirtualResolution { width: u32, height: u32 }, // 0x48004
    SetDepth(u32), // 0x48005
    SetPixelOrder(PixelOrder), // 0x48006
    SetAlphaMode(AlphaMode), // 0x48007
    SetVirtualOffset { x: u32, y: u32 }, // 48009
}

impl Request {
    fn write(&self, buf: &mut [u32; BUFFER_ENTRIES], c: usize) -> usize {
        let new_cursor = match self {
            Self::AllocateBuffer {..} => c + 5,
            Self::GetPitch => c + 4,
            Self::SetPhysicalResolution {..} => c + 5,
            Self::SetVirtualResolution {..} => c + 5,
            Self::SetDepth(..) => c + 4,
            Self::SetPixelOrder(..) => c + 4,
            Self::SetAlphaMode(..) => c + 4,
            Self::SetVirtualOffset {..} => c + 5,
        };
        assert!(new_cursor <= buf.len(), "Mailbox message buffer overflow ({} > {}). Too many requests?", new_cursor, buf.len());
        macro_rules! write_buf {
            ($($v: expr),*) => {{
                let mut c = c;
                $(buf[c] = $v; #[allow(unused_assignments)] c += 1;)*
            }};
        }
        match self {
            Self::AllocateBuffer { alignment } => write_buf![0x40001, 8, 8, *alignment, 0],
            Self::GetPitch => write_buf![0x40008, 4, 4, 0],
            Self::SetPhysicalResolution { width, height } => write_buf![0x48003, 8, 8, *width, *height],
            Self::SetVirtualResolution { width, height } => write_buf![0x48004, 8, 8, *width, *height],
            Self::SetDepth(depth) => write_buf![0x48005, 4, 4, *depth],
            Self::SetPixelOrder(pixel_order) => write_buf![0x48006, 4, 4, *pixel_order as _],
            Self::SetAlphaMode(mode) => write_buf![0x48007, 4, 4, *mode as _],
            Self::SetVirtualOffset { x, y } => write_buf![0x48009, 8, 8, *x, *y],
        }
        new_cursor
    }
}

#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Response {
    Nil,
    AllocateBuffer { base_address: *mut (), size: u32 }, // 40001,
    GetPitch(u32), // 0x40008
    SetPhysicalResolution { width: u32, height: u32 }, // 0x48003
    SetVirtualResolution { width: u32, height: u32 }, // 0x48004
    SetDepth(u32), // 0x48005
    SetPixelOrder(PixelOrder), // 0x48006
    SetAlphaMode(AlphaMode), // 0x48007
    SetVirtualOffset { x: u32, y: u32 }, // 48009
}

impl Response {
    fn read(buf: &[u32; BUFFER_ENTRIES], cursor: usize) -> (usize, Self) {
        match buf[cursor] {
            0x40001 => (cursor + 5, Self::AllocateBuffer {
                base_address: buf[cursor + 3] as usize as *mut (),
                size: buf[cursor + 4],
            }),
            0x40008 => (cursor + 4, Self::GetPitch(buf[cursor + 3] as _)),
            0x48003 => (cursor + 5, Self::SetPhysicalResolution {
                width: buf[cursor + 3],
                height: buf[cursor + 4],
            }),
            0x48004 => (cursor + 5, Self::SetVirtualResolution {
                width: buf[cursor + 3],
                height: buf[cursor + 4],
            }),
            0x48005 => (cursor + 4, Self::SetDepth(buf[cursor + 3])),
            0x48006 => (cursor + 4, Self::SetPixelOrder(unsafe { ::core::mem::transmute(buf[cursor + 3]) })),
            0x48007 => (cursor + 4, Self::SetAlphaMode(unsafe { ::core::mem::transmute(buf[cursor + 3]) })),
            0x48009 => (cursor + 5, Self::SetVirtualOffset {
                x: buf[cursor + 3],
                y: buf[cursor + 4],
            }),
            v => panic!("Unrecognized mailbox tag {:?}", v),
        }
    }
}

pub struct MailResponses {
    pub channel: Channel,
    responses: [Response; BUFFER_ENTRIES],
}

impl ::core::ops::Deref for MailResponses {
    type Target = [Response; BUFFER_ENTRIES];

    fn deref(&self) -> &Self::Target {
        &self.responses
    }
}

impl core::fmt::Debug for MailResponses {
    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        write!(f, "MailResponses({:?}) [", self.channel)?;
        for i in 0..self.responses.len() {
            if self.responses[i] == Response::Nil {
                break;
            }
            if i == 0 {
                write!(f, "{:?}", self.responses[i])?;
            } else {
                write!(f, ", {:?}", self.responses[i])?;
            }
        }
        write!(f, "]")
    }
}

pub struct Mail {
    pub channel: Channel,
    requests: [Option<Request>; 8],
    request_cursor: usize,
}

impl core::fmt::Debug for Mail {
    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        write!(f, "Mail({:?}) [", self.channel)?;
        for i in 0..self.requests.len() {
            if let Some(req) = self.requests[i] {
                if i == 0 {
                    write!(f, "{:?}", req)?;
                } else {
                    write!(f, ", {:?}", req)?;
                }
            } else {
                break;
            }
        }
        write!(f, "]")
    }
}

impl Mail {
    pub fn new(channel: Channel) -> Self {
        Self {
            channel,
            requests: [None; 8],
            request_cursor: 0,
        }
    }

    pub fn add(&mut self, req: Request) -> &mut Self {
        if self.request_cursor >= self.requests.len() {
            panic!("Too many (> 8) requests in a single mail");
        }
        self.requests[self.request_cursor] = Some(req);
        self.request_cursor += 1;
        self
    }

    pub fn send(self) -> Result<MailResponses, MailBoxError> {
        let mut buffer = MAILBOX_BUFFER.lock();
        // Set message buffer
        {
            // Set header
            buffer.0[0] = (BUFFER_ENTRIES * 4) as u32;
            buffer.0[1] = 0; // Process request
            let mut buffer_cursor = 2;
            for i in 0..self.requests.len() {
                match self.requests[i] {
                    Some(req) => buffer_cursor = req.write(&mut buffer.0, buffer_cursor),
                    None => break,
                }
            }
            // Set end tag
            buffer.0[buffer_cursor] = 0;
        }
        // Send
        buffer.send(self.channel as _)?;
        // Parse responses
        let mut responses = [Response::Nil; BUFFER_ENTRIES];
        {
            let mut buffer_cursor = 2;
            for i in 0..self.request_cursor {
                let (new_cursor, res) = Response::read(&buffer.0, buffer_cursor);
                responses[i] = res;
                buffer_cursor = new_cursor;
            }
        }
        let responses = MailResponses {
            channel: self.channel,
            responses,
        };
        // debug!("{:?}", responses);
        Ok(responses)
    }
}
