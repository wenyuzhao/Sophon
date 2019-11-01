target := aarch64-proton

build:
	@cargo xbuild --target $(target).json

run: build
	@qemu-system-aarch64 -M raspi3 -serial stdio -kernel ./target/$(target)/debug/proton
