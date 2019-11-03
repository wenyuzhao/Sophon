# A Raspberry PI Kernel Written in Rust

## Pre-requests

1. `qemu-system-aarch64` >= 2.12
2. Rustup nightly channel
3. [cargo-xbuild](https://github.com/rust-osdev/cargo-xbuild)
4. [cargo-binutils](https://github.com/rust-embedded/cargo-binutils)

## Build & Run

Under project root directory:

```
cargo xbuild --target aarch64-proton.json --features raspi3
cargo objcopy -- ./target/aarch64-proton/debug/proton -O binary ./kernel8.img
```

Then test the kernel with:

```
qemu-system-aarch64 -M raspi3 -serial stdio -kernel ./kernel8.img
```

**Alternative:** Simply run:

```
make run
```

## TODO

- [ ] Fill this todo list

## References

1. [Raspberry Pi Bare Bones Rust - OSDev](https://wiki.osdev.org/Raspberry_Pi_Bare_Bones_Rust)
2. [Mailbox Property Interface](https://github.com/raspberrypi/firmware/wiki/Mailbox-property-interface)
3. [Bare Metal Raspberry Pi 3 Tutorials](https://github.com/bztsrc/raspi3-tutorial)
4. [Bare Metal Raspberry Pi 3 Tutorials (Rust)](https://github.com/rust-embedded/rust-raspi3-OS-tutorials)
5. [Raspberry Pi Hardware Documents](https://github.com/raspberrypi/documentation/tree/master/hardware/raspberrypi)
6. [Learning OS dev using Linux kernel & Raspberry Pi](https://github.com/s-matyukevich/raspberry-pi-os)
