target := aarch64-proton
image := kernel8.img

build:
	@cargo xbuild --target $(target).json --features raspi3

run: build kernel8.img
	@qemu-system-aarch64 -M raspi3 -serial stdio -kernel ./kernel8.img

kernel8.img:
	@cargo objcopy -- ./target/$(target)/debug/proton -O binary ./kernel8.img