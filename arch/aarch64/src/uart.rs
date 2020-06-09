use crate::peripherals::*;
use proton_kernel::arch::*;



pub fn boot_time_log(s: &str) {
    fn putc(c: char) {
        let uart = UARTRegisters::get_low();
        while uart.fr.get() & (1 << 5) > 0 {}
        uart.dr.set(c as _);
    }
    for c in s.chars() {
        if c == '\n' {
            putc('\r')
        }
        putc(c)
    }
    putc('\r');
    putc('\n');
}

pub struct UART0;

impl UART0 {
    fn transmit_fifo_full() -> bool {
        let uart = UARTRegisters::get();
        uart.fr.get() & (1 << 5) > 0
    }
    
    fn receive_fifo_empty() -> bool {
        let uart = UARTRegisters::get();
        uart.fr.get() & (1 << 4) > 0
    }

    #[inline(never)]
    pub fn init() {
        let uart = UARTRegisters::get_low();
        let gpio = GPIORegisters::get_low();

        uart.cr.set(0);
        uart.icr.set(0);
        uart.ibrd.set(26);
        uart.fbrd.set(3);
        uart.lcrh.set((0b11 << 5) | (0b1 << 4));
        uart.cr.set((1 << 0) | (1 << 8) | (1 << 9));
            
        gpio.gpfsel1.set((0b100 << 12) | (0b100 << 15));
        gpio.gppud.set(0);
        wait_cycles(150);
        gpio.gppudclk0.set((1 << 14) | (1 << 15));
        wait_cycles(150);
        gpio.gppudclk0.set(0);
    }
}

impl AbstractLogger for UART0 {
    fn put(c: char) {
        while Self::transmit_fifo_full() {}
        let uart = UARTRegisters::get();
        uart.dr.set(c as _);
    }
    
    // fn get(&self) -> char {
    //     while self.receive_fifo_empty() {}
    //     self.mmio_read(Self::UART_DR) as _
    // }
}

fn wait_cycles(n: usize) {
    for _ in 0..n {
        unsafe { llvm_asm!("nop"); }
    }
}





// pub struct BootUART;

// impl BootUART {
//     fn transmit_fifo_full() -> bool {
//         let uart = UARTRegisters::get_low();
//         uart.fr.get() & (1 << 5) > 0
//     }

//     fn putc(&self, c: char) {
//         while Self::transmit_fifo_full() {}
//         let uart = UARTRegisters::get_low();
//         uart.dr.set(c as _);
//     }
// }

// use core::fmt::{self, Write};

// impl Write for BootUART {
//     fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
//         for c in s.chars() {
//             if c == '\n' {
//                 self.putc('\r')
//             }
//             self.putc(c)
//         }
//         Ok(())
//     }
// }
