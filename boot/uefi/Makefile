export profile ?= debug
export workspace = $(abspath $(dir $(abspath $(lastword $(MAKEFILE_LIST))))/../..)
export target = $(workspace)/target

lldb:
	rust-lldb -o "gdb-remote 1234" -- ../../target/aarch64-unknown-none/$(profile)/sophon

deploy: boot=/Volumes/BOOT
deploy:
	cp -r $(target)/_boot/EFI $(boot)/
	cp $(target)/_boot/EFI/BOOT/BOOTAA64.EFI $(boot)/boot.efi
	cp $(target)/_boot/sophon $(boot)/sophon
	cp $(target)/_boot/init.fs $(boot)/init.fs