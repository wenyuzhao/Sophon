#![no_std]

extern crate alloc;

use core::ops::Range;

use alloc::vec;
use alloc::vec::Vec;
use fdt_rs::base::*;
use fdt_rs::index::*;
use fdt_rs::prelude::*;
use memory::address::{Address, P};

pub struct DeviceTree<'buf, 'index> {
    _buf: &'buf [u8],
    _index_buf: Vec<u8>,
    index: DevTreeIndex<'index, 'buf>,
}

impl<'buf, 'index> DeviceTree<'buf, 'index> {
    pub fn new(buf: &'buf [u8]) -> Option<Self> {
        let devtree = unsafe { DevTree::new(buf) }.ok()?;
        let layout = DevTreeIndex::get_layout(&devtree).ok()?;

        // Allocate memory for the index.
        //
        // This could be performed without a dynamic allocation
        // if we allocated a static buffer or want to provide a
        // raw buffer into uninitialized memory.
        let mut index_buf = vec![0u8; layout.size() + layout.align()];
        let raw_slice = unsafe { &mut *(index_buf.as_mut_slice() as *mut [u8]) };

        // Create the index of the device tree.
        let index = DevTreeIndex::new(devtree, raw_slice).ok()?;
        Some(DeviceTree {
            _buf: buf,
            _index_buf: index_buf,
            index,
        })
    }

    pub fn compatible(&self, name: &str) -> Option<Node> {
        for n in self.index.nodes() {
            if let Some(compatible) = n.props().find(|p| p.name() == Ok("compatible")) {
                let mut strs = compatible.iter_str();
                while let Ok(Some(s)) = strs.next() {
                    if s == name {
                        return Some(Node { node: n });
                    }
                }
            }
        }
        None
    }

    pub fn cpus<'x>(&'x self) -> impl Iterator<Item = Node<'x, 'index, 'buf>> {
        self.index
            .nodes()
            .filter(|n| {
                n.props()
                    .find(|p| p.name() == Ok("device_type") && p.str() == Ok("cpu"))
                    .is_some()
            })
            .map(|n| Node { node: n })
    }
}

#[derive(Clone)]
pub struct Node<'a, 'index: 'a, 'buf: 'index> {
    node: DevTreeIndexNode<'a, 'index, 'buf>,
}

#[derive(Debug, Clone, Copy)]
pub struct CellSizes {
    pub address_cells: usize,
    pub size_cells: usize,
}

impl Default for CellSizes {
    fn default() -> Self {
        CellSizes {
            address_cells: 2,
            size_cells: 1,
        }
    }
}

impl<'a, 'index: 'a, 'buf: 'index> Node<'a, 'index, 'buf> {
    fn read_u32(buf: &mut &[u8]) -> u32 {
        let v = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]);
        *buf = &buf[4..];
        v
    }
    fn read_u64(buf: &mut &[u8]) -> u64 {
        let v = u64::from_be_bytes([
            buf[0], buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7],
        ]);
        *buf = &buf[8..];
        v
    }

    pub fn parent(&self) -> Option<Self> {
        self.node.parent().map(|node| Node { node })
    }

    pub fn ranges<'x>(
        &'x self,
    ) -> Option<impl Iterator<Item = (Address<P>, Address<P>, usize)> + 'x> {
        let parent_address_cells = self
            .parent()
            .map(|p| p.cell_sizes().address_cells)
            .unwrap_or_default();
        let CellSizes {
            address_cells,
            size_cells,
        } = self.cell_sizes();
        let ranges = self.node.props().find(|p| p.name() == Ok("ranges"))?;
        let mut buf = ranges.raw();
        Some(core::iter::from_fn(move || {
            if buf.len() == 0 {
                return None;
            }
            let child_addr = match address_cells {
                1 => Address::<P>::new(Self::read_u32(&mut buf) as usize),
                2 => Address::<P>::new(Self::read_u64(&mut buf) as usize),
                _ => unreachable!(),
            };
            let parent_addr = match parent_address_cells {
                1 => Address::<P>::new(Self::read_u32(&mut buf) as usize),
                2 => Address::<P>::new(Self::read_u64(&mut buf) as usize),
                _ => unreachable!(),
            };
            let size = match size_cells {
                1 => Self::read_u32(&mut buf) as usize,
                2 => Self::read_u64(&mut buf) as usize,
                _ => unreachable!(),
            };
            Some((child_addr, parent_addr, size))
        }))
    }

    pub fn regs<'x>(&'x self) -> Option<impl Iterator<Item = Range<Address<P>>> + 'x> {
        let CellSizes {
            address_cells,
            size_cells,
        } = self.parent().unwrap().cell_sizes();
        let prop = self.node.props().find(|p| p.name() == Ok("reg"))?;
        let mut buf = prop.raw();
        Some(core::iter::from_fn(move || {
            if buf.is_empty() {
                return None;
            }
            let addr = match address_cells {
                1 => Address::<P>::new(Self::read_u32(&mut buf) as usize),
                2 => Address::<P>::new(Self::read_u64(&mut buf) as usize),
                _ => unreachable!(),
            };
            let size = match size_cells {
                1 => Self::read_u32(&mut buf) as usize,
                2 => Self::read_u64(&mut buf) as usize,
                _ => unreachable!(),
            };
            Some(addr..addr + size)
        }))
    }

    pub fn translate(&self, a: Address<P>) -> Address<P> {
        let ranges = match self.ranges() {
            Some(x) => x,
            _ => return self.parent().map(|p| p.translate(a)).unwrap_or(a),
        };
        for (c, p, s) in ranges {
            if c <= a && a < c + s {
                let b = p + (a - c);
                return self.parent().map(|p| p.translate(b)).unwrap_or(b);
            }
        }
        a
    }

    pub fn cell_sizes(&self) -> CellSizes {
        let mut cell_sizes = match self.parent() {
            Some(p) => p.cell_sizes(),
            _ => CellSizes::default(),
        };
        for prop in self.node.props() {
            match prop.name().unwrap() {
                "#address-cells" => {
                    cell_sizes.address_cells = prop.u32(0).unwrap() as _;
                }
                "#size-cells" => {
                    cell_sizes.size_cells = prop.u32(0).unwrap() as _;
                }
                _ => {}
            }
        }
        cell_sizes
    }

    pub fn interrupt_cells(&self) -> usize {
        let prop = self
            .node
            .props()
            .find(|p| p.name() == Ok("#interrupt-cells"));

        if let Some(prop) = prop {
            prop.u32(0).ok().unwrap() as _
        } else {
            self.parent().map(|p| p.interrupt_cells()).unwrap_or(1)
        }
    }

    pub fn interrupts<'x>(&'x self) -> Option<impl Iterator<Item = (usize, usize)> + 'x> {
        let interrupt_cells = self.interrupt_cells();
        let prop = self.node.props().find(|p| p.name() == Ok("interrupts"))?;
        let mut buf = prop.raw();
        Some(core::iter::from_fn(move || {
            if buf.is_empty() {
                return None;
            }
            let is_spi = match interrupt_cells {
                1 => Self::read_u32(&mut buf) as usize,
                2 => Self::read_u64(&mut buf) as usize,
                _ => unreachable!(),
            } != 0;
            let irq = match interrupt_cells {
                1 => Self::read_u32(&mut buf) as usize,
                2 => Self::read_u64(&mut buf) as usize,
                _ => unreachable!(),
            };
            let ty = match interrupt_cells {
                1 => Self::read_u32(&mut buf) as usize,
                2 => Self::read_u64(&mut buf) as usize,
                _ => unreachable!(),
            };
            let irq_base = if is_spi { 16 } else { 32 };
            Some((irq_base + irq, ty))
        }))
    }
}
