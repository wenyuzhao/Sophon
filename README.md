# **Sophon** - A Raspberry Pi Kernel in Rust

An experimental _modular_-kernel written in Rust.

The name "Sophon" comes from the novel [_The Three-Body Problem_](https://en.wikipedia.org/wiki/The_Three-Body_Problem_(novel)).

# Getting Started

## Preparation


1. Install [rustup](https://rustup.rs/).
2. `qemu-system-aarch64` (optionally `gdb-multiarch` or `lldb` for debugging).
3. For debugging: LLVM tools (`lldb`, `llvm-objcopy` and `llvm-objdump`)
4. VSCode setup: install the [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=matklad.rust-analyzer) extension.


## Run on QEMU

```console
$ cargo dev run
```

## Run on a Raspberry Pi 4B

#### Prepare UEFI and bootable USB (once)

1. Prepare a USB drive with [UEFI firmware](https://github.com/pftf/RPi4).
2. Plug the usb to your Raspberry Pi and connect to a HDMI monitor (or using UART0).
3. Start Raspberry Pi and goto UEFI settings menu.
4. Navigate to `Device Manager` → `Raspberry Pi Configuration` → `Advanced Settings` and enable `ACPI + Device tree`

#### Install kernel

1. `cd boot/uefi`
2. `make deploy boot=/path/to/your/usb/directory`
3. Plug the usb to your Raspberry Pi and connect a serial cable to UART0 ports properly.
4. Use `screen` to connect to the serial device
   - e.g. `screen /dev/tty.usbserial 115200`.
5. Start Raspberry Pi

# Design

_The current plan is:_

Make the kernel as simple and small as possible. Instead of following the micro-kernel approach that puts each kernel component as a isolated user process, Sophon tries to treat the components as kernel modules. This is expected to have higher performance than micro-kernels due to the absence of context switches and message passing overheads.

# TODO

### Boot

- [x] Make the kernel boot on AArch64 QEMU (UEFI)
- [x] Make the kernel boot on a real Raspberry Pi 4B (UEFI)
- [x] Setup EL1 virtual memory
- [x] Load kernel as relocatable ELF
- [x] Start kernel at Exception Level 1
- [ ] UEFI Network boot
- [ ] U-boot support

### Kernel

- [x] Initialize drivers based on a device tree
- [x] Basic interrupt handler support
- [x] Kernel heap allocation
- [x] Timer interrupts
- [x] Scheduling / Context switch
- [x] Syscalls support
- [x] `Log` syscall (output to *UART*, for user process debugging)
- [x] Kernel Modules
- [x] Module-defined syscalls (_Module calls_)
- [x] VFS module and Root-FS
- [ ] Memory management module; `mmap` and `munmap` syscalls
- [ ] File system modules like fat32
- [x] Process management module
- [x] Process and multi-threading
- [x] Driver interface based on modules
- [ ] SMP support

### User Space

- [ ] Properly trap and handle Stack-overflow exception
- [x] Launch init process in privileged mode
- [x] Launch init process in user mode
- [x] TTY
- [ ] Update/release ref-counted pages after process exit
- [ ] Port gcc/libc/rustc

### Architectures

- [x] AArch64
- [ ] X86_64
- [ ] X86
- [ ] ARMv6-M (RTOS)

### Others

- [x] Unit / integration tests
- [x] Continuous integration (based on GitHub Actions)

# References

1. [Raspberry Pi Bare Bones Rust - OSDev](https://wiki.osdev.org/Raspberry_Pi_Bare_Bones_Rust)
2. [Mailbox Property Interface](https://github.com/raspberrypi/firmware/wiki/Mailbox-property-interface)
3. [Bare Metal Raspberry Pi 3 Tutorials](https://github.com/bztsrc/raspi3-tutorial)
4. [Bare Metal Raspberry Pi 3 Tutorials (Rust)](https://github.com/rust-embedded/rust-raspi3-OS-tutorials)
5. [Raspberry Pi Hardware Documents](https://github.com/raspberrypi/documentation/tree/master/hardware/raspberrypi)
6. [Learning OS dev using Linux kernel & Raspberry Pi](https://github.com/s-matyukevich/raspberry-pi-os)
7. [ARM Quad-A7 Documentation (for timer configuration)](https://github.com/raspberrypi/documentation/blob/master/hardware/raspberrypi/bcm2836/QA7_rev3.4.pdf)
8. [Circle - A C++ bare metal programming env for RPi](https://github.com/rsta2/circle)
9. [PanicOS - A simple x86 operating system with graphical user space](https://github.com/JasonBrave/PanicOS)
