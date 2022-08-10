#![feature(format_args_nl)]
#![feature(default_alloc_error_handler)]
#![feature(box_syntax)]
#![feature(step_trait)]
#![feature(const_trait_impl)]
#![feature(const_mut_refs)]
#![feature(core_intrinsics)]
#![feature(generic_associated_types)]
#![no_std]

#[allow(unused)]
#[macro_use]
extern crate log;
extern crate alloc;

pub mod queue;
pub mod virtio_mmio;

use core::mem::MaybeUninit;

use kernel_module::{kernel_module, KernelModule, SERVICE};
use memory::page::{Frame, Size4K};
use syscall::NetRequest;
use virtio_mmio::{IpAddress, MacAddress};

use crate::virtio_mmio::{VirtIOHeader, VirtIONetDevice};

#[kernel_module]
pub static mut VIRTIO_NET: VirtIONet = VirtIONet {
    mac: MacAddress::BROADCAST,
    net: None,
    irq: 0,
};

pub struct VirtIONet {
    mac: MacAddress,
    net: Option<VirtIONetDevice>,
    irq: usize,
}

unsafe impl Send for VirtIONet {}
unsafe impl Sync for VirtIONet {}

#[repr(C)]
#[derive(Default)]
struct ICMP<T: Default> {
    kind: u8,
    code: u8,
    checksum: u16,
    identifier: u16,
    sequence_number: u16,
    payload: T,
}

#[repr(C)]
#[derive(Debug, Default)]
struct IpV4<T> {
    version_ihl: u8,
    dscp_ecn: u8,
    length: u16,
    ident: u16,
    flags_fragment: u16,
    ttl: u8,
    protocol: u8,
    checksum: u16,
    source: u32,
    destination: u32,
    payload: T,
}

impl<T> IpV4<T> {
    fn checksum(&self) -> u16 {
        let ptr = self as *const Self as *const u16;
        let buf = unsafe { core::slice::from_raw_parts(ptr, 10) };
        let mut sum = 0u32;
        for x in buf {
            sum += x.to_be() as u32;
            if sum > 0xFFFF {
                sum = (sum >> 16) + (sum & 0xFFFF);
            }
        }
        (!(sum & 0xffff) & 0xffff) as u16
    }
}

#[repr(C)]
#[derive(Default, Debug)]
struct Arp {
    hw: u16,
    proto: u16,
    hwsize: u8,
    protosize: u8,
    op: u16,
    sender_mac: MacAddress,
    sender_ip: IpAddress,
    target_mac: MacAddress,
    target_ip: IpAddress,
}

#[repr(C)]
#[derive(Debug, Default)]
struct Ether<T> {
    destination: MacAddress,
    source: MacAddress,
    kind: u16,
    payload: T,
}

#[repr(C)]
#[derive(Debug, Default)]
struct UDP<T> {
    pub source_port: u16,
    pub destination_port: u16,
    pub length: u16,
    pub checksum: u16,
    pub payload: T,
}

#[repr(C)]
#[derive(Debug)]
struct DHCP {
    pub op: u8,
    pub htype: u8,
    pub hlen: u8,
    pub hops: u8,
    pub xid: u32,
    pub secs: u16,
    pub flags: u16,
    pub ciaddr: u32,
    pub yiaddr: u32,
    pub siaddr: u32,
    pub giaddr: u32,
    pub chaddr: [u8; 16],
    pub sname: [u8; 64],
    pub file: [u8; 128],
    pub magic: u32,
    pub options: [u8; 32],
}

struct Payload([u8; 48]);

impl Default for Payload {
    fn default() -> Self {
        Payload([0; 48])
    }
}

impl KernelModule for VirtIONet {
    type ModuleRequest<'a> = NetRequest;

    fn init(&'static mut self) -> anyhow::Result<()> {
        let devtree = SERVICE.get_device_tree().unwrap();
        for node in devtree.iter_compatible("virtio,mmio") {
            let regs = node.regs().unwrap().next().unwrap();
            let offset = regs.start - Frame::<Size4K>::align(regs.start);
            let start = Frame::containing(node.translate(regs.start));
            let end = Frame::new(Frame::<Size4K>::align_up(node.translate(regs.end)));
            let pages = SERVICE.map_device_pages(start..end);
            let start = pages.start.start() + offset;
            if let Some(header) = VirtIOHeader::from(start) {
                let irq = node.interrupts().unwrap().next().unwrap().0;
                SERVICE.set_irq_handler(irq, box || {
                    println!("Net IRQ");
                    let net = unsafe { VIRTIO_NET.net.as_mut().unwrap() };
                    let status = net.header.interrupt_status.get();
                    net.header.interrupt_ack.set(status);
                    println!("Net IRQ status: {:x}", status);
                    let arp = net.recv_one::<Ether<IpV4<UDP<DHCP>>>>();
                    println!("{:?}", arp);
                    0
                });
                SERVICE.enable_irq(irq);
                let net = VirtIONetDevice::init(header);
                self.mac = net.config.mac();
                self.net = Some(net);
                self.irq = irq;
            }
        }
        Ok(())
    }

    fn handle_module_call<'a>(
        &self,
        _privileged: bool,
        _request: Self::ModuleRequest<'a>,
    ) -> isize {
        unsafe { VIRTIO_NET.init_net() }
        0
    }
}

impl VirtIONet {
    fn init_net(&mut self) {
        // self.arp_ask(IpAddress(172, 217, 167, 110));
        self.dhcp_discover();
    }

    fn dhcp_discover(&mut self) {
        let mut dhcp = DHCP {
            op: 1,
            htype: 1,
            hlen: 6,
            hops: 0,
            xid: 0x1337, // ???
            secs: 0,
            flags: 0,
            ciaddr: 0,
            yiaddr: 0,
            siaddr: 0,
            giaddr: 0,
            chaddr: [0; 16],
            sname: [0; 64],
            file: [0; 128],
            magic: 0x63825363u32.to_be(),
            options: [0; 32],
        };
        for i in 0..6 {
            dhcp.chaddr[i] = self.mac[i];
        }
        for (i, v) in [99, 130, 83, 88, 53, 1, 3].iter().enumerate() {
            dhcp.options[i] = *v;
        }
        let udp = UDP {
            source_port: 0x44u16.to_be(),
            destination_port: 0x43u16.to_be(),
            length: (core::mem::size_of::<UDP<DHCP>>() as u16 - 32 + 7).to_be(),
            checksum: 0,
            payload: dhcp,
        };
        let mut ipv4 = IpV4 {
            version_ihl: (0x4 << 4) | (0x5 << 0),
            dscp_ecn: 0,
            length: (core::mem::size_of::<IpV4<UDP<DHCP>>>() as u16 - 32 + 7).to_be(),
            ident: 1u16.to_be(),
            flags_fragment: 0x0040,
            ttl: 0x40,
            protocol: 17, // udp
            checksum: 0u16.to_be(),
            source: 0u32.to_be(),
            destination: 0xFFFFFFFFu32.to_be(),
            payload: udp,
        };
        ipv4.checksum = ipv4.checksum();
        let eth = Ether {
            destination: MacAddress::BROADCAST,
            source: self.mac,
            kind: 0x0800u16.to_be(),
            payload: ipv4,
        };
        println!("dhcp sending");
        self.net.as_mut().unwrap().send(eth);
        println!("dhcp sent");
        let page = SERVICE.alloc_pages(1).unwrap().start;
        unsafe {
            page.zero();
        }
        let p: &mut Ether<IpV4<UDP<DHCP>>> = unsafe { page.start().as_mut() };
        self.net
            .as_mut()
            .unwrap()
            .revc_sync::<[u8; 4096]>(unsafe { page.start().as_mut() });
        println!("dhcp received {:x?} {:?}", p, unsafe {
            page.start().as_mut::<[u8; 4096]>()
        });
    }

    fn arp_ask(&mut self, ip: IpAddress) -> MacAddress {
        assert_eq!(core::mem::size_of::<Arp>(), 28);
        let mut arp = Arp::default();
        arp.hw = 0x0100;
        arp.proto = 0x0008;
        arp.hwsize = 0x06; // sizeof(deviceMAC);
        arp.protosize = 0x04; // sizeof(deviceIP);
        arp.op = 0x0100;
        arp.sender_mac = self.mac;
        arp.target_ip = ip;
        let mut eth = Ether::<Arp>::default();
        eth.source = self.mac;
        eth.destination = MacAddress::BROADCAST;
        eth.kind = 0x0608;
        eth.payload = arp;
        println!("send");
        self.net.as_mut().unwrap().send(eth);
        println!("sent");
        MacAddress::BROADCAST
    }
}
