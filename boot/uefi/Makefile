include ../../proton/common.mk

build:
	$(call build, proton)
	cargo build $(args)
	mkdir -p $(target)/_boot $(target)/_boot/EFI/BOOT
# Copy kernel
	cp $(target)/aarch64-unknown-none/$(profile)/proton $(target)/_boot/
# Copy efi bootloader. Use BOOTX64.EFI for x86_64.
	cp $(target)/aarch64-proton-uefi/$(profile)/proton-boot-uefi.efi $(target)/_boot/EFI/BOOT/BOOTAA64.EFI
	cp -Rf ../dtbs/* $(target)/_boot/dtbs/

run: build
	cargo run -- $(qemu_args)

lldb:
	rust-lldb -o "gdb-remote 1234" -- ../../target/aarch64-unknown-none/$(profile)/proton
