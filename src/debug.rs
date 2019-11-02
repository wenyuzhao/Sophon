use core::fmt;
use core::fmt::Write;
use spin::Mutex;
use core::intrinsics::volatile_load;
use core::intrinsics::volatile_store;
use crate::gpio::PERIPHERAL_BASE;


#[doc(hidden)]
pub fn _println(args: fmt::Arguments) {
    let mut write = UART_WRITER.lock();
    write.write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => ({
        $crate::debug::_println(format_args_nl!($($arg)*))
    });
}

lazy_static! {
    static ref UART_WRITER: Mutex<UART> = Mutex::new(UART);
}

struct UART;

#[allow(unused)]
impl UART {
    const UART_DR: usize = PERIPHERAL_BASE + 0x201000;
    const UART_FR: usize = PERIPHERAL_BASE + 0x201018;

    fn mmio_write(&self, reg: usize, val: u32) {
        unsafe { volatile_store(reg as *mut u32, val) }
    }
    
    fn mmio_read(&self, reg: usize) -> u32 {
        unsafe { volatile_load(reg as *const u32) }
    }
    
    fn transmit_fifo_full(&self) -> bool {
        self.mmio_read(Self::UART_FR) & (1 << 5) > 0
    }
    
    fn receive_fifo_empty(&self) -> bool {
        self.mmio_read(Self::UART_FR) & (1 << 4) > 0
    }
    
    fn putc(&self, c: u8) {
        while self.transmit_fifo_full() {}
        self.mmio_write(Self::UART_DR, c as u32);
    }
    
    fn getc(&self) -> u8 {
        while self.receive_fifo_empty() {}
        self.mmio_read(Self::UART_DR) as u8
    }

    fn delay(&self, i: usize) {
        for _ in 0..i {
            unsafe { asm!("nop"); }
        }
    }
}

impl Write for UART {
    fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
        for b in s.bytes() {
            self.putc(b)
        }
        Ok(())
    }
}

