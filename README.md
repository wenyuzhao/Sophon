# A Raspberry PI Kernel Written in Rust

## Pre-requests

1. `qemu-system-aarch64` >= 2.12
2. Rustup nightly channel
3. [cargo-xbuild](https://github.com/rust-osdev/cargo-xbuild)

## Build

Under project root directory:
```
cargo xbuild --target aarch64_proton.json
```
Then test the kernel with:
```
qemu-system-aarch64 -M raspi3 -serial stdio -kernel ./target/aarch64_proton/debug/proton
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
