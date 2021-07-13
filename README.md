# A Raspberry Pi Kernel Written in Rust

## Pre-requests

1. [rustup](https://rustup.rs/)
2. LLVM tools (`llvm-objcopy` and `llvm-objdump`)

**VSCode setup**

1. Install the [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=matklad.rust-analyzer) extension

**Test/debug with QEMU and GDB**

1. `qemu-system-aarch64` >= 2.12
2. `gdb-multiarch`

## Build & Run

```console
$ cd boot/uefi
$ make run
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
- [ ] ~~`Fork` syscall (and handle copy-on-write pages after `fork()`)~~
  - Probably we only some `execve`-like syscalls.
- [ ] `ProcessExit` syscall
- [ ] Update/release ref-counted pages after process exit
- [x] Inter Process Communication
- [ ] Memory map related syscalls (`mmap`, `munmap`)
- [ ] *May need to port gcc/libc/rustc at this point*
- [ ] Multi-core support
- [ ] Design & implement a driver interface
- [ ] VFS and init.rd
- [ ] Basic FAT32 FS support
- [ ] Basic graphics support
- [ ] *Other necessary components for a kernel?*

**Supported architectures:**

- [x] AArch64
- [ ] X86_64
- [ ] X86
- [ ] ARMv6-M (RTOS)

## References

1. [Raspberry Pi Bare Bones Rust - OSDev](https://wiki.osdev.org/Raspberry_Pi_Bare_Bones_Rust)
2. [Mailbox Property Interface](https://github.com/raspberrypi/firmware/wiki/Mailbox-property-interface)
3. [Bare Metal Raspberry Pi 3 Tutorials](https://github.com/bztsrc/raspi3-tutorial)
4. [Bare Metal Raspberry Pi 3 Tutorials (Rust)](https://github.com/rust-embedded/rust-raspi3-OS-tutorials)
5. [Raspberry Pi Hardware Documents](https://github.com/raspberrypi/documentation/tree/master/hardware/raspberrypi)
6. [Learning OS dev using Linux kernel & Raspberry Pi](https://github.com/s-matyukevich/raspberry-pi-os)
7. [ARM Quad-A7 Documentation (for timer configuration)](https://github.com/raspberrypi/documentation/blob/master/hardware/raspberrypi/bcm2836/QA7_rev3.4.pdf)
8. [Circle - A C++ bare metal programming env for RPi](https://github.com/rsta2/circle)
