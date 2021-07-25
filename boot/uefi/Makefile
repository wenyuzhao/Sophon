include ../../proton/common.mk

build:
	$(call build, proton)
	cargo build $(args)
	llvm-objdump --section-headers --source -d ../../target/aarch64-proton-uefi/$(profile)/proton-boot-uefi.efi > ../../target/aarch64-proton-uefi/$(profile)/proton-boot-uefi.s 2> /dev/null
	mkdir -p $(target)/_boot $(target)/_boot/EFI/BOOT
# Copy kernel
	cp $(target)/aarch64-unknown-none/$(profile)/proton $(target)/_boot/
# Copy efi bootloader. Use BOOTX64.EFI for x86_64.
	cp $(target)/aarch64-proton-uefi/$(profile)/proton-boot-uefi.efi $(target)/_boot/EFI/BOOT/BOOTAA64.EFI

run: build
	cargo run -- $(qemu_args)

lldb:
	rust-lldb -o "gdb-remote 1234" -- ../../target/aarch64-unknown-none/$(profile)/proton

deploy: boot=/Volumes/BOOT
deploy: build
	cp -r $(target)/_boot/EFI "$(boot)/"
	cp $(target)/_boot/EFI/BOOT/BOOTAA64.EFI "$(boot)/boot.efi"
	cp ./.cargo/startup.nsh "$(boot)/startup.nsh"
	cp $(target)/_boot/proton "$(boot)/proton"