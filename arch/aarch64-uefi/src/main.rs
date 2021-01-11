#![no_std]
#![no_main]
#![feature(global_asm)]
#![feature(llvm_asm)]
#![feature(alloc_error_handler)]
#![feature(format_args_nl)]
#![feature(core_intrinsics)]
#![feature(box_syntax)]
#![feature(new_uninit)]
#![feature(never_type)]
#![feature(step_trait_ext)]
#![feature(const_fn_transmute)]
#![feature(const_in_array_repeat_expressions)]

extern crate alloc;
extern crate bitflags;
extern crate fdt_rs;
use core::{mem, panic::PanicInfo};
use alloc::vec;
use cortex_a::{asm, regs::*, barrier};
use device_tree::Node;
use uefi::{Guid, prelude::*, table::{Runtime, boot::MemoryDescriptor}};
#[macro_use] mod log;
mod arch;
mod exception;
mod interrupt;
mod timer;
mod bootimage;
mod idle;
mod heap;
mod mm;
mod gic;
mod context;
mod drivers;

static mut BOOT_SYSTEM_TABLE: Option<SystemTable<Boot>> = None;

fn boot_system_table() -> &'static SystemTable<Boot> {
    unsafe { BOOT_SYSTEM_TABLE.as_ref().unwrap() }
}

use proton_kernel::{AbstractKernel, boot_driver::BootDriver, kernel_process};
use proton_kernel::scheduler::round_robin::RoundRobinScheduler;
use arch::AArch64;



#[global_allocator]
static ALLOCATOR: heap::GlobalAllocator = heap::GlobalAllocator::new();

static KERNEL: Kernel = Kernel {
    global: <Kernel as AbstractKernel>::INITIAL_GLOBAL,
};



pub struct Kernel {
    global: <Self as AbstractKernel>::Global,
}

impl AbstractKernel for Kernel {
    type Arch = AArch64;
    type Scheduler = RoundRobinScheduler<Self>;

    fn global() -> &'static Self::Global {
        &KERNEL.global
    }
}

// fn get_fdt() -> Option<*const u8> {
//     for entry in system_table().config_table() {
//         log!("GUID: {}", entry.guid);
//         if entry.guid == Guid::from_values(0xb1b621d5, 0xf19c, 0x41a5, 0x830b, [0xd9, 0x15, 0x2c, 0x69, 0xaa, 0xe0]) {
//             return Some(entry.address as _);
//         }
//     }
//     log!("NOT FOUND {}", Guid::from_values(0xb1b621d5, 0xf19c, 0x41a5, 0x830b, [0xd9, 0x15, 0x2c, 0x69, 0xaa, 0xe0]));
//     None
// }
extern crate device_tree;
static DEVICE_TREE: &'static [u8] = include_bytes!("../qemu-virt.dtb");


use fdt_rs::prelude::*;
use fdt_rs::base::*;
// fn dump_node(n: &Node, indent: usize) {
//     s = (0..indent).
// }

unsafe extern fn setup_vbar(ptr: u64) {

    log!("efi_main: {:?}", efi_main as *const fn());
    log!("handle_exception: {:?}", exception::handle_exception as *const fn());
    log!("exception_handlers: {:?}", exception::exception_handlers as *const fn());
    log!("exception_handlers real: {:#x}", ptr);
    VBAR_EL1.set(ptr);
    barrier::isb(barrier::SY);
}

#[no_mangle]
pub extern "C" fn efi_main(image: Handle, st: SystemTable<Boot>) -> Status {
    unsafe { BOOT_SYSTEM_TABLE = Some(st.unsafe_clone()); }
    ALLOCATOR.init();
    // let max_mmap_size =
    //     st.boot_services().memory_map_size() + 8 * mem::size_of::<MemoryDescriptor>();
    // let mut mmap_storage = vec![0; max_mmap_size].into_boxed_slice();
    // unsafe { SYSTEM_TABLE = Some(st.exit_boot_services(image, mmap_buf)); }
    // st.exit_boot_services(image, mmap_buf).ou
    let t = device_tree::DeviceTree::load(DEVICE_TREE).unwrap();


    {
        let mut uart = drivers::uart::UART.lock();
        uart.init_with_device_tree(&t);
        uart.putchar('@');
        uart.putchar('\n');
    }
    unsafe { BOOT_SYSTEM_TABLE = None; }
    log!("Start exit_boot_services");

    {
        let max_mmap_size =
        st.boot_services().memory_map_size() + 8 * mem::size_of::<MemoryDescriptor>();
    let mut mmap_storage = vec![0; max_mmap_size].into_boxed_slice();
    let (st, _iter) = st
        .exit_boot_services(image, &mut mmap_storage[..])
        .expect_success("Failed to exit boot services");
    }

    log!("Finish exit_boot_services");

    let intc = t.find("/intc@8000000").unwrap();
    let reg = intc.prop_raw("reg").unwrap();
    log!("{:?}", intc);

    drivers::gic::GIC.init_with_device_tree(&t);



    // get_fdt();
    unsafe {
        // unsafe { llvm_asm!("msr daifset, #2") };
        // unsafe { llvm_asm!("msr daifclr, #2") };

    log!("[boot: enable all co-processors]");
    llvm_asm!("msr cpacr_el1, $0"::"r"(0xfffffff));
    setup_vbar(exception::exception_handlers as *const fn() as u64 - 0x1000);
        crate::mm::paging::setup_ttbr();
    }



    unsafe { BOOT_SYSTEM_TABLE = None; }
    unsafe {
        *(0xdeadbeed as *mut u8) = 0;
    }
    log!("CurrentEL: {}", CurrentEL.get() >> 2);

    loop {}
    // log!("CurrentEL: {}", CurrentEL.get() >> 2);
    // log!("Firmware vendor: {}", boot_system_table().firmware_vendor());
    // log!("Firmware revision: {:?}", boot_system_table().firmware_revision());
    // log!("UEFI revision: {:?}", boot_system_table().uefi_revision());
    // let mut buf = [0u8; 4096];
    // let (k, i) = boot_system_table().boot_services().memory_map(&mut buf).unwrap().unwrap();
    // log!("mmap key: {:?}", k);
    // for i in i {
    //     log!("> mem phys_start={:?} virt_start={:?} pages={:?}", i.phys_start as *const (), i.virt_start as *const (), i.page_count);
    //     log!("{:?}", i);
    // }
    Kernel::start();
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    log!("{}", info);
    loop {}
}

#[no_mangle]
pub extern "C" fn __chkstk() {}

#[alloc_error_handler]
fn alloc_error_handler(layout: ::alloc::alloc::Layout) -> ! {
    panic!("Allocation error: {:?}", layout)
}