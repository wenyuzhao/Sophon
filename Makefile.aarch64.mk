# Run on QEMU
#     `make run`
# Build & deploy to a real raspi4 (Windows/WSL)
#     `raspi=4 make kernel objdump copy-win`

kernel_target = aarch64-kernel
user_target = aarch64-proton
device ?= raspi3-qemu
# Optional: release=1
# Optional: dint=1

# Derived configurations
profile = $(if $(release),release,debug)
debug_interrupts = $(if $(dint),-d int)
output_elf = target/$(kernel_target)/$(profile)/proton
output_img = target/$(kernel_target)/$(profile)/kernel8.img
output_init_elf = target/$(user_target)/$(profile)/init
qemu_command = qemu-system-aarch64 -display none -M raspi3 -serial stdio
qemu_debug_interrupts = $(if $(dint),-d int)
qemu_gdb_server = $(if $(gdb),-s -S)



kernel: init FORCE
	@cd kernel && cargo xbuild --no-default-features $(if $(release), --release) --target $(kernel_target).json --features device-$(device)
	@llvm-objcopy --strip-all $(output_elf) -O binary $(output_img)

init: FORCE
	@cd init && cargo xbuild --no-default-features $(if $(release), --release) --target $(user_target).json
	@cp target/$(user_target)/$(profile)/init target/init

run: device=raspi3-qemu
run: kernel objdump
	@$(qemu_command) $(qemu_debug_interrupts) $(qemu_gdb_server) -kernel $(output_img)

objdump:
	@llvm-objdump --source -d $(output_elf) > kernel.S
	@llvm-objdump --source -d $(output_init_elf) > init.S

gdb:
	@gdb-multiarch -quiet "$(output_elf)" -ex "set arch aarch64" -ex "target remote :1234"

clean:
	@cargo clean
	@cd init && cargo clean
	@rm ./Cargo.lock ./kernel.S ./init.S ./kernel8.img

copy-win:
	@PowerShell.exe -Command "copy $(output_img) D:/"

