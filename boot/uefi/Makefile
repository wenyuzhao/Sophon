include ../../proton/common.mk

build:
	$(call build, proton)
	cargo build $(args)
	mkdir -p $(target)/_boot
	cp $(target)/aarch64-unknown-none/$(profile)/proton $(target)/_boot/
	cp $(target)/aarch64-proton-uefi/debug/proton-boot-uefi.efi $(target)/_boot/
	cp ./.cargo/startup.nsh $(target)/_boot/

run: build
	cargo run -- $(qemu_args)

lldb:
	rust-lldb -o "gdb-remote 1234" -- ../../target/aarch64-unknown-none/$(profile)/proton
