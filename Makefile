target := aarch64-proton
image := kernel8.img
raspi ?= 3
cross-gdb = ~/tools/gdb/gdb/gdb

build:
	@cargo xbuild --target $(target).json --features raspi$(raspi)
	@cargo objcopy -- ./target/$(target)/debug/proton -O binary ./kernel8.img

run: build kernel8.img
	@qemu-system-aarch64 -M raspi3 -serial stdio -kernel ./kernel8.img -d int

objdump:
	aarch64-linux-gnu-objdump -d ./target/$(target)/debug/proton > kernel.S

debug: build kernel8.img objdump
	@qemu-system-aarch64 -M raspi3 -serial stdio -kernel ./kernel8.img  -d int -s -S

gdb:
	$(cross-gdb) "target/aarch64-proton/debug/proton" -ex "set arch aarch64" -ex "target remote :1234"
