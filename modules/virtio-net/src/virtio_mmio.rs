use crate::queue::{VirtQueue, VirtQueueDescFlags};
use alloc::{boxed::Box, fmt};
use bitflags::bitflags;
use core::{ops::Index, ptr::read_volatile, sync::atomic::Ordering};
use memory::{
    address::{Address, P, V},
    volatile::Volatile,
};

#[repr(C)]
pub struct VirtIOHeader {
    pub magic: Volatile<u32>,
    pub version: Volatile<u32>,
    pub device_id: Volatile<u32>,
    pub vendor_id: Volatile<u32>,
    pub device_features: Volatile<u32>,
    pub device_features_sel: Volatile<u32>,
    _1: [u32; 2],
    pub driver_features: Volatile<u32>,
    pub driver_features_sel: Volatile<u32>,
    pub guest_page_size: Volatile<u32>,
    _2: [u32; 1],
    pub queue_sel: Volatile<u32>,
    pub queue_num_max: Volatile<u32>,
    pub queue_num: Volatile<u32>,
    pub queue_align: Volatile<u32>,
    pub queue_pfn: Volatile<u32>,
    pub queue_ready: Volatile<u32>,
    _3: [u32; 2],
    pub queue_notify: Volatile<u32>,
    _4: [u32; 3],
    pub interrupt_status: Volatile<u32>,
    pub interrupt_ack: Volatile<u32>,
    _5: [u32; 2],
    pub status: Volatile<DeviceStatus>,
    _6: [u32; 3],
    pub queue_desc_low: Volatile<u32>,
    pub queue_desc_high: Volatile<u32>,
    _7: [u32; 2],
    pub queue_avail_low: Volatile<u32>,
    pub queue_avail_high: Volatile<u32>,
    _8: [u32; 2],
    pub queue_used_low: Volatile<u32>,
    pub queue_used_high: Volatile<u32>,
    _9: [u32; 1],
    pub shm_sel: Volatile<u32>,
    pub shm_sel_len_low: Volatile<u32>,
    pub shm_sel_len_high: Volatile<u32>,
    pub shm_sel_base_low: Volatile<u32>,
    pub shm_sel_base_high: Volatile<u32>,
    _10: [u32; 15],
    pub config_generation: Volatile<u32>,
}

impl VirtIOHeader {
    pub fn from(addr: Address) -> Option<&'static mut Self> {
        let header = unsafe { addr.as_mut::<VirtIOHeader>() };
        assert_eq!(header.magic.get(), 0x7472_6976);
        // assert_eq!(header.version.get(), 1);
        if header.device_id.get() == 0 {
            None
        } else {
            Some(header)
        }
    }

    pub fn start_initialization(&mut self) {
        self.status.set(DeviceStatus::Reset);
        self.status.set(DeviceStatus::Acknowledge);
        self.status.set(DeviceStatus::Driver);
    }

    pub fn finish_initialization(&mut self) {
        // self.status.set(DeviceStatus::FeaturesOk.into());
        self.status.set(DeviceStatus::DriverOk.into());
    }

    pub fn get_device_features<T: From<u64>>(&mut self) -> T {
        self.device_features_sel.set(0);
        let mut features = self.device_features.get() as u64;
        self.device_features_sel.set(1);
        features += (self.device_features.get() as u64) << 32;
        features.into()
    }

    pub fn set_device_features(&mut self, features: impl Into<u64>) {
        let features = features.into();
        self.driver_features_sel.set(0);
        self.driver_features.set(features as u32);
        self.driver_features_sel.set(1);
        self.driver_features.set((features >> 32) as u32);
    }

    pub fn update_device_features<T: From<u64> + Into<u64>>(&mut self, f: impl Fn(T) -> T) {
        let features = self.get_device_features::<T>();
        self.set_device_features(f(features));
    }

    pub fn device_type(&self) -> Option<DeviceType> {
        let id = self.device_id.get();
        if id >= 1 && id <= 10 {
            Some(unsafe { core::mem::transmute(id as u8) })
        } else {
            None
        }
    }

    pub fn set_queue(&mut self, index: u32, queue: &VirtQueue) {
        self.queue_sel.set(index);
        assert_eq!(self.queue_ready.get(), 0);
        let max_size = self.queue_num_max.get();
        assert_ne!(max_size, 0);
        self.queue_num.set(queue.size() as _);
        let write_addr = |addr: Address<P>, low: &mut Volatile<u32>, high: &mut Volatile<u32>| {
            let v = addr.as_usize() as u64;
            low.set(v as u32);
            high.set((v >> 32) as u32);
        };
        write_addr(
            queue.desc_start(),
            &mut self.queue_desc_low,
            &mut self.queue_desc_high,
        );
        write_addr(
            queue.avail_start(),
            &mut self.queue_avail_low,
            &mut self.queue_avail_high,
        );
        write_addr(
            queue.used_start(),
            &mut self.queue_used_low,
            &mut self.queue_used_high,
        );
        self.queue_ready.set(1);
    }

    pub fn notify(&mut self, queue_index: u32) {
        self.queue_notify.set(queue_index);
    }
}

#[allow(unused)]
#[repr(u8)]
#[derive(Debug, Eq, PartialEq)]
pub enum DeviceType {
    Network = 1,
    Block = 2,
    Console = 3,
    EntropySource = 4,
    MemoryBallooning = 5,
    IoMemory = 6,
    RPMSG = 7,
    SCSIHost = 8,
    NinePTransport = 9,
    Mac80211WLAN = 10,
}

#[repr(C)]
#[derive(Eq, PartialEq, Clone, Copy, Default)]
pub struct IpAddress(pub u8, pub u8, pub u8, pub u8);

impl IpAddress {}

impl fmt::Debug for IpAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}.{}", self.0, self.1, self.2, self.3)
    }
}

#[repr(transparent)]
#[derive(Eq, PartialEq, Clone, Copy, Default)]
pub struct MacAddress([u8; 6]);

impl MacAddress {
    pub const BROADCAST: Self = MacAddress([0xff; 6]);
}

impl Index<usize> for MacAddress {
    type Output = u8;

    fn index(&self, i: usize) -> &Self::Output {
        &self.0[i]
    }
}

impl fmt::Debug for MacAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

#[repr(C)]
pub struct VirtIONetConfig {
    pub mac: Volatile<[u8; 6]>,
    pub status: Volatile<u16>,
    pub max_virtqueue_pairs: Volatile<u16>,
    pub mtu: Volatile<u16>,
}

impl VirtIONetConfig {
    pub fn from(header: &VirtIOHeader) -> &'static Self {
        unsafe { (Address::<V>::from(header) + 0x100usize).as_ref() }
    }

    pub fn mac(&self) -> MacAddress {
        MacAddress(self.mac.get())
    }

    pub fn status(&self) -> u16 {
        self.status.get()
    }
}

/// The device status field provides a simple low-level indication of the completed steps of this sequence. It’s most useful to imagine it hooked up to traffic lights on the console indicating the status of each device.
#[allow(unused)]
#[bitflags(u32)]
pub enum DeviceStatus {
    Reset = 0,
    /// Indicates that the guest OS has found the device and recognized it as a valid virtio device.
    Acknowledge = 1,
    /// Indicates that the guest OS knows how to drive the device. Note: There could be a significant (or infinite) delay before setting this bit. For example, under Linux, drivers can be loadable modules.
    Driver = 2,
    /// Indicates that something went wrong in the guest, and it has given up on the device. This could be an internal error, or the driver didn’t like the device for some reason, or even a fatal error during device operation.
    Failed = 128,
    /// Indicates that the driver has acknowledged all the features it understands, and feature negotiation is complete.
    FeaturesOk = 8,
    /// Indicates that the driver is set up and ready to drive the device.
    DriverOk = 4,
    /// Indicates that the device has experienced an error from which it can’t recover.
    DeviceNeedsReset = 64,
}

#[allow(unused)]
#[bitflags(u64)]
pub enum NetworkDeviceFeatures {
    /// Device handles packets with partial checksum. This “checksum offload” is a common feature on modern network cards.
    CSUM = 1 << 0,
    /// Driver handles packets with partial checksum.
    GUEST_CSUM = 1 << 1,
    /// Control channel offloads reconfiguration support.
    CTRL_GUEST_OFFLOADS = 1 << 2,
    /// Device maximum MTU reporting is supported. If offered by the device, device advises driver about the value of its maximum MTU. If negotiated, the driver uses mtu as the maximum MTU value.
    MTU = 1 << 3,
    /// Device has given MAC address.
    MAC = 1 << 5,
    /// Driver can receive TSOv4.
    GUEST_TSO4 = 1 << 7,
    /// Driver can receive TSOv6.
    GUEST_TSO6 = 1 << 8,
    /// Driver can receive TSO with ECN.
    GUEST_ECN = 1 << 9,
    /// Driver can receive UFO.
    GUEST_UFO = 1 << 10,
    /// Device can receive TSOv4.
    HOST_TSO4 = 1 << 11,
    /// Device can receive TSOv6.
    HOST_TSO6 = 1 << 12,
    /// Device can receive TSO with ECN.
    HOST_ECN = 1 << 13,
    /// Device can receive UFO.
    HOST_UFO = 1 << 14,
    /// Driver can merge receive buffers.
    MRG_RXBUF = 1 << 15,
    /// Configuration status field is available.
    STATUS = 1 << 16,
    /// Control channel is available.
    CTRL_VQ = 1 << 17,
    /// Control channel RX mode support.
    CTRL_RX = 1 << 18,
    /// Control channel VLAN filtering.
    CTRL_VLAN = 1 << 19,
    /// Driver can send gratuitous packets.
    GUEST_ANNOUNCE = 1 << 21,
    /// Device supports multiqueue with automatic receive steering.
    MQ = 1 << 22,
    /// Set MAC address through control channel.
    CTRL_MAC_ADDR = 1 << 23,
    /// Device can process duplicated ACKs and report number of coalesced segments and duplicated ACKs
    RSC_EXT = 1 << 61,
    /// Device may act as a standby for a primary device with the same MAC address.
    STANDBY = 1 << 62,
}

#[repr(C)]
pub struct VirtIONetHeader {
    pub flags: NetHeaderFlags,
    pub gso_type: NetHeaderGsoType,
    pub hdr_len: u16,
    pub gso_size: u16,
    pub csum_start: u16,
    pub csum_offset: u16,
    pub num_buffers: u16,
}

impl Default for VirtIONetHeader {
    fn default() -> Self {
        Self {
            flags: 0.into(),
            gso_type: 0.into(),
            hdr_len: 0,
            gso_size: 0,
            csum_start: 0,
            csum_offset: 0,
            num_buffers: 0,
        }
    }
}

#[allow(unused)]
#[bitflags(u8)]
pub enum NetHeaderFlags {
    NEED_CSUM = 1,
    DATA_VALID = 2,
    RSC_INFO = 4,
}

impl Default for NetHeaderFlags {
    fn default() -> Self {
        Self::from(0)
    }
}

#[allow(unused)]
#[bitflags(u8)]
pub enum NetHeaderGsoType {
    None = 0,
    TcpV4 = 1,
    Udp = 2,
    TcpV6 = 4,
    Ecn = 0x80,
}

impl Default for NetHeaderGsoType {
    fn default() -> Self {
        Self::from(0)
    }
}

pub struct VirtIONetDevice {
    pub header: &'static mut VirtIOHeader,
    pub config: &'static VirtIONetConfig,
    rx: &'static mut VirtQueue,
    tx: &'static mut VirtQueue,
}

impl VirtIONetDevice {
    pub fn init(header: &'static mut VirtIOHeader) -> Self {
        header.start_initialization();
        header.update_device_features::<NetworkDeviceFeatures>(|features| {
            features & NetworkDeviceFeatures::MAC
        });
        header.status.set(DeviceStatus::FeaturesOk.into());
        if !header.status.get().contains(DeviceStatus::FeaturesOk) {
            panic!("Coudln't set net features");
        }
        let (tx, rx) = (VirtQueue::new(), VirtQueue::new());
        tx.avail.flags = 1;
        header.set_queue(0, rx);
        header.set_queue(1, tx);

        let config = VirtIONetConfig::from(header);
        log!("MAC: {:?}", config.mac());
        header.finish_initialization();

        let mut me = Self {
            header,
            config,
            tx,
            rx,
        };

        me.init_rx();

        me
    }

    fn init_rx(&mut self) {
        for i in 0..self.rx.size() / 2 {
            let header = Box::leak(Box::new(VirtIONetHeader::default()));
            let data = Box::leak(Box::new([0u8; 4096]));
            let d1 = self
                .rx
                .push(header, VirtQueueDescFlags::Write | VirtQueueDescFlags::Next);
            let d2 = self.rx.push(&data, VirtQueueDescFlags::Write);
            self.rx.desc[d1].next = d2 as _;
            self.rx.desc_virtual_ptrs[d1] = Address::from(header);
            self.rx.desc_virtual_ptrs[d2] = Address::from(data);
            self.rx.update_avail(i as _);
        }
        self.header.notify(0);
    }

    pub fn recv_one<T>(&mut self) -> &'static T {
        println!("used {:?}", self.rx.used.index);
        let d1 = self.rx.used.ring[self.rx.used.index as usize].id;
        let len = self.rx.used.ring[self.rx.used.index as usize].len;
        println!("d1 {} len {:?}", d1, len);
        let d2 = self.rx.desc[d1 as usize].next;
        let addr = self.rx.desc_virtual_ptrs[d2 as usize];
        println!("addr {:?}", addr);
        unsafe { addr.as_ref() }
    }

    pub fn send<T>(&mut self, data: T) {
        let header = VirtIONetHeader::default();
        let d1 = self.tx.push(&header, VirtQueueDescFlags::Next);
        let d2 = self.tx.push(&data, VirtQueueDescFlags::None);
        self.tx.desc[d1].next = d2 as _;

        core::sync::atomic::fence(Ordering::SeqCst);
        self.tx.update_avail(d1 as _);
        core::sync::atomic::fence(Ordering::SeqCst);
        self.header.notify(1);
        core::sync::atomic::fence(Ordering::SeqCst);
        loop {
            core::hint::spin_loop();
            unsafe {
                let used_index = read_volatile(&self.tx.used.index);
                let avail_index = read_volatile(&self.tx.avail.index);
                if used_index == avail_index {
                    break;
                }
            }
        }
    }
}
