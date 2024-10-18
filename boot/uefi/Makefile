export profile ?= debug
export workspace = $(abspath $(dir $(abspath $(lastword $(MAKEFILE_LIST))))/../..)
export target = $(workspace)/target

lldb:
	rust-lldb -o "gdb-remote 1234" -- ../../target/_out/sophon

gdb:
	gdb-multiarch -ex='target remote localhost:1234' ../../target/_out/sophon

copy-files: boot=/Volumes/BOOT
copy-files:
	cp -r $(target)/_boot/EFI $(boot)/
	cp $(target)/_boot/sophon $(boot)/sophon
	cp $(target)/_boot/init.fs $(boot)/init.fs
	sync

build-debug:
	cd ../.. && cargo dev build

deploy-debug: build-debug copy-files

build-release:
	cd ../.. && cargo dev build --release

deploy: build-release copy-files
