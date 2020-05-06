target = aarch64-unknown-none
kernel_src = $(project)/arch/aarch64
kernel_elf = $(project)/target/$(target)/$(profile)/proton
kernel_img = $(project)/target/$(target)/$(profile)/kernel8.img
kernel_rust_flags = -C link-arg=-T$(kernel_src)/aarch64.ld
qemu_command = qemu-system-aarch64 -display none -M raspi3 -serial stdio -drive file=test.img,if=sd,format=raw
qemu_debug_interrupts = $(if $(dint),-d int)
qemu_gdb_server = $(if $(gdb),-s -S)



arch_user_program:
	cd $(path) && cargo build --target $(target)
	cd $(path) && cargo objdump --target $(target) -- --source -d > $(project)/target/$(target)/$(profile)/$(strip $(name)).s 2>&1

arch_kernel:
	@cd $(kernel_src) && RUSTFLAGS="$(kernel_rust_flags)" cargo build --target $(target) --features device-$(strip $(device)),$(strip $(features))
	@cd $(kernel_src) && RUSTFLAGS="$(kernel_rust_flags)" cargo objcopy --target $(target) -- --strip-all -O binary $(kernel_img) > /dev/null 2>&1 
	@cd $(kernel_src) && cargo objdump --target $(target) -- --source -d > $(kernel_elf).s 2>&1

arch_run: device=raspi3-qemu
arch_run: kernel
	@$(qemu_command) $(qemu_debug_interrupts) $(qemu_gdb_server) -kernel $(kernel_img)

arch_gdb:
	lldb --arch aarch64 --file $(kernel_elf) --one-line "gdb-remote 1234"
