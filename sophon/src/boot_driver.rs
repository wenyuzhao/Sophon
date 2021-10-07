use crate::memory::kernel::{KERNEL_HEAP, KERNEL_MEMORY_MAPPER};
use core::{marker::PhantomData, ops::ControlFlow};
use fdt::{node::FdtNode, Fdt};
use memory::page_table::{PageFlags, PageFlagsExt};
use memory::{
    address::{Address, P},
    page::*,
};

pub trait BootDriver {
    const COMPATIBLE: &'static [&'static str];
    fn init(&mut self, node: &FdtNode, parent: Option<&FdtNode>);
    fn map_device_page(frame: Frame) -> Page {
        let page = KERNEL_HEAP.virtual_allocate::<Size4K>(1).start;
        KERNEL_MEMORY_MAPPER.map_fixed(page, frame, PageFlags::device());
        page
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
}

impl<T: BootDriver> DynBootDriver for T {
    fn init(&mut self, fdt: &Fdt) {
        if let Some((node, parent)) = self.find_compatible(fdt, Self::COMPATIBLE) {
            self.init(&node, parent.as_ref())
        }
    }
}

pub trait DynBootDriver {
    fn init(&mut self, fdt: &Fdt);
    fn find_compatible<'a>(
        &mut self,
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
}

pub fn init(device_tree: &Fdt, drivers: &mut [&mut dyn DynBootDriver]) {
    for driver in drivers {
        driver.init(&device_tree);
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
