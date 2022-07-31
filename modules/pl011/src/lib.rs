#![feature(format_args_nl)]
#![feature(default_alloc_error_handler)]
#![no_std]

#[macro_use]
extern crate log;
extern crate alloc;

use core::{marker::PhantomData, ops::ControlFlow};

use dev::{DevRequest, Device};
use spin::RwLock;

use fdt::{node::FdtNode, Fdt};
use kernel_module::{kernel_module, KernelModule, SERVICE};
use memory::{
    address::{Address, P},
    page::Frame,
    volatile::Volatile,
};

#[kernel_module]
pub static PL011_MODULE: PL011 = PL011 {
    uart: RwLock::new(core::ptr::null_mut()),
};

unsafe impl Send for PL011 {}
unsafe impl Sync for PL011 {}

pub struct PL011 {
    pub uart: RwLock<*mut UART0>,
}

impl PL011 {
    fn find_compatible<'a>(
        &self,
        fdt: &'a Fdt<'a>,
        compatible: &'static [&'static str],
    ) -> Option<(FdtNode<'a, 'a>, Option<FdtNode<'a, 'a>>)> {
        let visitor = FdtNodeVisitor::<'a, 'a, _, _>::new(move |n, p| {
            if n.compatible().is_some()
                && n.compatible()
                    .unwrap()
                    .all()
                    .find(|s| compatible.contains(s))
                    .is_some()
            {
                ControlFlow::Break((n, p))
            } else {
                ControlFlow::Continue(())
            }
        });
        visitor.visit(fdt)
    }

    fn translate_address(a: Address<P>, node: &FdtNode) -> Address<P> {
        if let Some(ranges) = node.property("ranges") {
            for i in (0..ranges.value.len()).step_by(16) {
                let v = &ranges.value[i..i + 16];
                let child_start =
                    Address::<P>::new(u32::from_be_bytes([v[0], v[1], v[2], v[3]]) as usize);
                let parent_start =
                    Address::<P>::new(u32::from_be_bytes([v[8], v[9], v[10], v[11]]) as usize);
                let size = u32::from_be_bytes([v[12], v[13], v[14], v[15]]) as usize;
                if a >= child_start && a < child_start + size {
                    return parent_start + (a - child_start);
                }
            }
        }
        a
    }

    pub fn init_uart0(&self, node: FdtNode, parent: FdtNode) {
        let mut uart_frame =
            Address::<P>::new(node.reg().unwrap().next().unwrap().starting_address as usize);
        uart_frame = Self::translate_address(uart_frame, &parent);
        let uart_page = SERVICE.map_device_page(Frame::new(uart_frame));
        let uart = unsafe { &mut *(uart_page.start().as_mut_ptr() as *mut UART0) };
        uart.init();
        *self.uart.write() = uart;
    }

    fn uart(&self) -> &mut UART0 {
        unsafe { &mut **self.uart.read() }
    }
}

impl KernelModule for PL011 {
    fn init(&'static self) -> anyhow::Result<()> {
        log!("Hello, PL011!");
        let fdt = SERVICE.get_device_tree().unwrap();
        let (node, parent) = self.find_compatible(&fdt, &["arm,pl011"]).unwrap();
        self.init_uart0(node, parent.unwrap());
        log!("register_device");
        kernel_module::module_call(
            "dev",
            &DevRequest::RegisterDev(&(self as &'static dyn Device)),
        );
        // log!("Please type");
        // loop {
        //     print!("> ");
        //     let c = self.uart().getchar(true);
        //     println!("{:?}", c);
        //     if c == Some('x') {
        //         break;
        //     }
        // }
        Ok(())
    }
}

impl Device for PL011 {
    fn name(&self) -> &'static str {
        "tty.serial"
    }

    fn read(&self, _offset: usize, buf: &mut [u8]) -> usize {
        for i in 0..buf.len() {
            buf[i] = match self.uart().getchar(false) {
                Some(c) => c as u8,
                None => return i,
            };
        }
        0
    }
}

#[repr(C)]
pub struct UART0 {
    pub dr: Volatile<u32>,     // 0x00
    pub rsrecr: Volatile<u32>, // 0x04
    _0: [u8; 16],              // 0x08
    pub fr: Volatile<u32>,     // 0x18,
    _1: [u8; 4],               // 0x1c,
    pub ilpr: Volatile<u32>,   // 0x20,
    pub ibrd: Volatile<u32>,   // 0x24,
    pub fbrd: Volatile<u32>,   // 0x28,
    pub lcrh: Volatile<u32>,   // 0x2c,
    pub cr: Volatile<u32>,     // 0x30,
    pub ifls: Volatile<u32>,   // 0x34,
    pub imsc: Volatile<u32>,   // 0x38,
    pub ris: Volatile<u32>,    // 0x3c,
    pub mis: Volatile<u32>,    // 0x40,
    pub icr: Volatile<u32>,    // 0x44,
    pub dmacr: Volatile<u32>,  // 0x48,
}

impl UART0 {
    // fn transmit_fifo_full(&self) -> bool {
    //     self.fr.get() & (1 << 5) != 0
    // }

    fn receive_fifo_empty(&self) -> bool {
        self.fr.get() & (1 << 4) != 0
    }

    fn getchar(&mut self, block: bool) -> Option<char> {
        if self.receive_fifo_empty() {
            if !block {
                return None;
            }
            while self.receive_fifo_empty() {
                core::hint::spin_loop();
            }
        }
        let mut ret = self.dr.get() as u8 as char;
        if ret == '\r' {
            ret = '\n';
        }
        // if ret as u8 == 127 {
        //     ret = 0x8u8 as _;
        // }
        Some(ret)
    }

    // fn putchar(&mut self, c: char) {
    //     while self.transmit_fifo_full() {}
    //     self.dr.set(c as u8 as u32);
    // }

    fn init(&mut self) {
        self.cr.set(0);
        self.icr.set(0);
        self.ibrd.set(26);
        self.fbrd.set(3);
        self.lcrh.set((0b11 << 5) | (0b1 << 4));
        self.cr.set((1 << 0) | (1 << 8) | (1 << 9));
    }
}

pub struct FdtNodeVisitor<
    'b,
    'a: 'b,
    B,
    F: 'a + 'b + Fn(FdtNode<'b, 'a>, Option<FdtNode<'b, 'a>>) -> ControlFlow<B, ()>,
>(pub F, PhantomData<(&'b B, &'a B)>);

impl<'b, 'a: 'b, B, F: Fn(FdtNode<'b, 'a>, Option<FdtNode<'b, 'a>>) -> ControlFlow<B, ()>>
    FdtNodeVisitor<'b, 'a, B, F>
{
    pub fn new(f: F) -> Self {
        Self(f, PhantomData)
    }
    pub fn visit(&self, fdt: &'b Fdt<'a>) -> Option<B> {
        match self.visit_node(fdt.find_node("/").unwrap(), None) {
            ControlFlow::Break(x) => Some(x),
            _ => None,
        }
    }

    fn visit_node(
        &self,
        node: FdtNode<'b, 'a>,
        parent: Option<FdtNode<'b, 'a>>,
    ) -> ControlFlow<B, ()> {
        self.0(node, parent)?;
        for child in node.children() {
            self.visit_node(child, Some(node))?;
        }
        ControlFlow::Continue(())
    }
}
