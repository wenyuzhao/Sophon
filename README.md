# A Raspberry PI Kernel Written in Rust

## Pre-requests

1. `qemu-system-aarch64` >= 2.12
2. Rustup nightly channel
3. [cargo-xbuild](https://github.com/rust-osdev/cargo-xbuild)
4. [cargo-binutils](https://github.com/rust-embedded/cargo-binutils)

## Build & Run

```bash
make build # This will produce a `kernel8.img` under project root directory
make run # Test the kernel with QEMU
```

## Design

The current plan is:
Make the kernel as simple as possible. So we will likely to make a MINIX-like
micro kernel. Then we can throw most tasks, including drivers, fs to the user
space.

BTW, it is almost impossible to take care of performance for now...

## TODO

- [ ] Make the kernel boot on a real Raspberry PI
- [x] Boot kernel into Exception Level 0
- [x] Setup kernel virtual memory
- [x] Basic interrupt handler support
- [x] Kernel heap allocation
- [ ] Properly trap and handle Stack-overflow exception
- [x] Enter to usermode
- [x] Syscalls
- [ ] Usermode memory map
- [ ] Fork syscall (either from kernel or user mode)
- [ ] ProcessExit syscall (either from kernel or user mode)
- [x] Timer interrupts
- [x] Scheduling/Context switch
- [ ] *May need to port GCC/Rustc/libc at this point*
- [ ] Init process
- [ ] Multi-core support
- [ ] Design & implement a driver interface
- [ ] Basic FAT32 FS support (to load init.d from /boot)
- [ ] Virtual File System
- [ ] *Other necessary components for a kernel?*

## References

1. [Raspberry Pi Bare Bones Rust - OSDev](https://wiki.osdev.org/Raspberry_Pi_Bare_Bones_Rust)
2. [Mailbox Property Interface](https://github.com/raspberrypi/firmware/wiki/Mailbox-property-interface)
3. [Bare Metal Raspberry Pi 3 Tutorials](https://github.com/bztsrc/raspi3-tutorial)
4. [Bare Metal Raspberry Pi 3 Tutorials (Rust)](https://github.com/rust-embedded/rust-raspi3-OS-tutorials)
5. [Raspberry Pi Hardware Documents](https://github.com/raspberrypi/documentation/tree/master/hardware/raspberrypi)
6. [Learning OS dev using Linux kernel & Raspberry Pi](https://github.com/s-matyukevich/raspberry-pi-os)
7. [ARM Quad-A7 Documentation (for timer configuration)](https://github.com/raspberrypi/documentation/blob/master/hardware/raspberrypi/bcm2836/QA7_rev3.4.pdf)
