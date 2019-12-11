use core::fmt;
use core::fmt::Write;
use core::intrinsics::volatile_load;
use core::intrinsics::volatile_store;


#[doc(hidden)]
#[inline(never)]
pub fn _print(args: fmt::Arguments) {
    unsafe {
        UART_WRITER.write_fmt(args).unwrap();
    }
}


pub fn log(s: &str) {
    unsafe {
        UART_WRITER.println(s);
    }
}

#[macro_export]
macro_rules! boot_log {
    ($($arg:tt)*) => {{
        $crate::debug_boot::_print(format_args_nl!($($arg)*))
    }};
}

fn wait_cycles(n: usize) {
    for _ in 0..n {
        unsafe { asm!("nop"); }
    }
}

pub static mut UART_WRITER: UART = UART;
pub struct UART;

impl UART {    
    #[cfg(feature="raspi3")]
    const BASE: usize = 0x3F201000;
    #[cfg(feature="raspi4")]
    const BASE: usize = 0xFE201000;
    const UART_DR: *mut u32   = (Self::BASE + 0x00) as _;
    const UART_FR: *mut u32   = (Self::BASE + 0x18) as _;
    const UART_IBRD: *mut u32 = (Self::BASE + 0x24) as _;
    const UART_FBRD: *mut u32 = (Self::BASE + 0x28) as _;
    const UART_LCRH: *mut u32 = (Self::BASE + 0x2C) as _;
    const UART_CR: *mut u32   = (Self::BASE + 0x30) as _;
    const UART_ICR: *mut u32  = (Self::BASE + 0x44) as _;
    #[cfg(feature="raspi3")] const GPIO_BASE: usize = 0x3F200000;
    #[cfg(feature="raspi4")] const GPIO_BASE: usize = 0xFE200000;
    const GPFSEL1: *mut u32 = (Self::GPIO_BASE + 0x4) as _;
    const GPPUD: *mut u32 = (Self::GPIO_BASE + 0x94) as _;
    const GPPUDCLK0: *mut u32 = (Self::GPIO_BASE + 0x98) as _;
    const GPPUDCLK1: *mut u32 = (Self::GPIO_BASE + 0x9C) as _;

    #[cfg(feature="raspi3")]
    const PBASE: usize = 0x3F000000;
    #[cfg(feature="raspi4")]
    const PBASE: usize = 0xFE000000;
    const AUX_ENABLES: *mut u32     = (Self::PBASE + 0x00215004) as _;
    const AUX_MU_IO_REG: *mut u32   = (Self::PBASE + 0x00215040) as _;
    const AUX_MU_IER_REG: *mut u32  = (Self::PBASE + 0x00215044) as _;
    const AUX_MU_IIR_REG: *mut u32  = (Self::PBASE + 0x00215048) as _;
    const AUX_MU_LCR_REG: *mut u32  = (Self::PBASE + 0x0021504C) as _;
    const AUX_MU_MCR_REG: *mut u32  = (Self::PBASE + 0x00215050) as _;
    const AUX_MU_LSR_REG: *mut u32  = (Self::PBASE + 0x00215054) as _;
    const AUX_MU_MSR_REG: *mut u32  = (Self::PBASE + 0x00215058) as _;
    const AUX_MU_SCRATCH: *mut u32  = (Self::PBASE + 0x0021505C) as _;
    const AUX_MU_CNTL_REG: *mut u32 = (Self::PBASE + 0x00215060) as _;
    const AUX_MU_STAT_REG: *mut u32 = (Self::PBASE + 0x00215064) as _;
    const AUX_MU_BAUD_REG: *mut u32 = (Self::PBASE + 0x00215068) as _;

    #[inline(never)]
    pub fn init() {
        unsafe {
            *Self::UART_CR = 0;
            *Self::UART_ICR = 0;
            *Self::UART_IBRD = 26;
            *Self::UART_FBRD = 3;
            *Self::UART_LCRH = (0b11 << 5) | (0b1 << 4);
            *Self::UART_CR = (1 << 0) | (1 << 8) | (1 << 9);

            // let mut x: u32 = *Self::GPFSEL1;
            // x &= !(7<<12);                   // clean gpio14
	        // x |= 2<<12;                      // set alt5 for gpio14
	        // x &= !(7<<15);                   // clean gpio15
            // x |= 2<<15;                      // set alt5 for gpio15
            // *Self::GPFSEL1 = x;
            *Self::GPFSEL1 = (0b100 << 12) | (0b100 << 15);
            *Self::GPPUD = 0;
            wait_cycles(150);
            *Self::GPPUDCLK0 = (1 << 14) | (1 << 15);
            wait_cycles(150);
            *Self::GPPUDCLK0 = 0;


            // *Self::AUX_ENABLES = 1;                   //Enable mini uart (this also enables access to it registers)
	        // *Self::AUX_MU_CNTL_REG = 0;               //Disable auto flow control and disable receiver and transmitter (for now)
	        // *Self::AUX_MU_IER_REG = 0;                //Disable receive and transmit interrupts
	        // *Self::AUX_MU_LCR_REG = 3;                //Enable 8 bit mode
	        // *Self::AUX_MU_MCR_REG = 0;                //Set RTS line to be always high
	        // *Self::AUX_MU_BAUD_REG = 541;             //Set baud rate to 115200
	        // *Self::AUX_MU_CNTL_REG = 3;               //Finally, enable transmitter and receiver
        }
    }

    
    fn mmio_write(&self, reg: *mut u32, val: u32) {
        unsafe { volatile_store(reg as *mut u32, val) }
    }
    
    
    fn mmio_read(&self, reg: *mut u32) -> u32 {
        unsafe { volatile_load(reg as *const u32) }
    }

    
    fn transmit_fifo_full(&self) -> bool {
        // self.mmio_read(Self::AUX_MU_LSR_REG) & 0x20 == 0
        self.mmio_read(Self::UART_FR) & (1 << 5) > 0
    }
    
    
    fn putc(&self, c: u8) {
        while self.transmit_fifo_full() {}
        // self.mmio_write(Self::AUX_MU_IO_REG, c as u32);
        self.mmio_write(Self::UART_DR, c as u32);
    }

    
    pub fn println(&self, s: &str) {
        for c in s.bytes() {
            if c == '\n' as u8 {
                self.putc('\r' as u8)
            }
            self.putc(c)
        }
        self.putc('\r' as u8);
        self.putc('\n' as u8);
    }
}

impl Write for UART {
    fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
        for c in s.bytes() {
            if c == '\n' as u8 {
                self.putc('\r' as u8)
            }
            self.putc(c)
        }
        Ok(())
    }
}



