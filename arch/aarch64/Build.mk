target = aarch64-unknown-none
kernel_src = $(project)/arch/aarch64
kernel_elf = $(project)/target/$(target)/$(profile)/proton
kernel_img = $(project)/target/$(target)/$(profile)/kernel8.img
kernel_rust_flags = -C link-arg=-T$(kernel_src)/aarch64.ld
qemu_command = qemu-system-aarch64 -display none -M raspi3 -serial stdio -drive file=test.img,if=sd,format=raw
qemu_debug_interrupts = $(if $(dint),-d int)
qemu_gdb_server = $(if $(gdb),-s -S)
user_target = aarch64-proton
user_target_json = $(project)/proton/$(user_target).json


arch-user-program: # args: name, path
	@cd $(path) && cargo xbuild --target $(user_target_json) $(cargo_profile_flag)
	@cp $(project)/target/$(user_target)/$(profile)/$(strip $(name)) $(project)/target/$(user_target)/$(strip $(name))
	@llvm-objdump --section-headers --source -d $(project)/target/$(user_target)/$(profile)/$(strip $(name)) > $(project)/target/$(user_target)/$(profile)/$(strip $(name)).s

arch-kernel: # args: device (raspi4 / raspi3-qemu), features
	@cd $(kernel_src) && RUSTFLAGS="$(kernel_rust_flags)" cargo build $(cargo_profile_flag) --target $(target) --no-default-features --features device-$(strip $(device)),$(strip $(features))
	@llvm-objcopy --strip-all $(kernel_elf) -O binary $(kernel_img)
	@llvm-objdump --section-headers --source -d $(kernel_elf) > $(kernel_elf).s

arch-run: device=raspi3-qemu
arch-run: kernel
	@$(qemu_command) $(qemu_debug_interrupts) $(qemu_gdb_server) -kernel $(kernel_img)

arch-gdb:
	# lldb --arch aarch64 --file $(kernel_elf) --one-line "gdb-remote 1234"
	gdb -quiet "$(kernel_elf)" -ex "set arch aarch64" -ex "target remote :1234"

raspi4-build: device=raspi4
raspi4-build: kernel

raspi4-mac: raspi4-build
	cp $(kernel_img) /Volumes/boot/

raspi4-win: raspi4-build
	PowerShell.exe -Command "copy $(kernel_img) D:/"