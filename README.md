# A Raspberry Pi Kernel Written in Rust

## Pre-requests

**Build**
1. Rustup nightly channel
2. Rustup target `aarch64-unknown-linux-gnu`
3. [cargo-xbuild](https://github.com/rust-osdev/cargo-xbuild)
4. LLVM Tools (`llvm-objcopy` and `llvm-objdump`)

**Test/debug with QEMU**
1. `qemu-system-aarch64` >= 2.12
2. `gdb-multiarch`

## Build & Run

```bash
make kernel # This will produce `target/aarch64-kernel/debug/kernel8.img`
make run # Test the kernel with QEMU
```

## Design

The current plan is:
Make the kernel as simple as possible. So we will likely to make a MINIX-like
micro kernel. Then we can throw most tasks, including drivers, fs to the user
space.

BTW, it is almost impossible to take care of performance for now...

## TODO

- [x] Make the kernel boot on a real Raspberry Pi
- [x] Start kernel at Exception Level 2
- [x] Setup kernel virtual memory
- [x] Basic interrupt handler support
- [x] Kernel heap allocation
- [ ] Properly trap and handle Stack-overflow exception
- [x] Launch init process in privileged mode
- [x] Launch init process in user mode
- [x] Timer interrupts
- [x] Scheduling/Context switch
- [x] Syscalls support
- [x] `Log` syscall (output to *UART*, for user process debugging)
- [x] `Fork` syscall (and handle copy-on-write pages after `fork()`)
- [ ] `ProcessExit` syscall
- [ ] Update/release ref-counted pages after process exit
- [x] Inter Process Communication
- [ ] Memory map related syscalls (`mmap`, `munmap`)
- [ ] *May need to port GCC/Rustc/libc at this point*
- [ ] Multi-core support
- [ ] Design & implement a driver interface
- [ ] Basic FAT32 FS support (to load init.d from /boot)
- [ ] Virtual File System
- [ ] *Other necessary components for a kernel?*

**Supported architectures:**

- [x] AArch64
- [ ] ARMv6-M (RTOS)
- [ ] X86
- [ ] X86_64

## References

1. [Raspberry Pi Bare Bones Rust - OSDev](https://wiki.osdev.org/Raspberry_Pi_Bare_Bones_Rust)
2. [Mailbox Property Interface](https://github.com/raspberrypi/firmware/wiki/Mailbox-property-interface)
3. [Bare Metal Raspberry Pi 3 Tutorials](https://github.com/bztsrc/raspi3-tutorial)
4. [Bare Metal Raspberry Pi 3 Tutorials (Rust)](https://github.com/rust-embedded/rust-raspi3-OS-tutorials)
5. [Raspberry Pi Hardware Documents](https://github.com/raspberrypi/documentation/tree/master/hardware/raspberrypi)
6. [Learning OS dev using Linux kernel & Raspberry Pi](https://github.com/s-matyukevich/raspberry-pi-os)
7. [ARM Quad-A7 Documentation (for timer configuration)](https://github.com/raspberrypi/documentation/blob/master/hardware/raspberrypi/bcm2836/QA7_rev3.4.pdf)
8. [Circle - A C++ bare metal programming env for RPi](https://github.com/rsta2/circle)
