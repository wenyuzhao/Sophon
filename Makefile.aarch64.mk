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
user_target_json = $(CURDIR)/proton/$(user_target).json
debug_interrupts = $(if $(dint),-d int)
output_elf = target/$(kernel_target)/$(profile)/proton
output_img = target/$(kernel_target)/$(profile)/kernel8.img
qemu_command = qemu-system-aarch64 -display none -M raspi3 -serial stdio -drive file=test.img,if=sd,format=raw
qemu_debug_interrupts = $(if $(dint),-d int)
qemu_gdb_server = $(if $(gdb),-s -S)



kernel: init FORCE
	@cd arch/aarch64 && $(cargo_xbuild) --target $(kernel_target).json --features device-$(device)
	@llvm-objcopy --strip-all $(output_elf) -O binary $(output_img)
	@llvm-objdump --source -d $(output_elf) > $(output_elf).s

user_process: FORCE
	cd $(process_path) && $(cargo_xbuild) --target $(user_target_json)
	@llvm-objdump --source -D target/$(user_target)/$(profile)/$(process_name) > target/$(user_target)/$(profile)/$(process_name).s

drivers: FORCE
	@make user_process process_path=drivers/emmc process_name=emmc

init: FORCE
	@make user_process process_path=init process_name=init

run: device=raspi3-qemu
run: kernel
	$(qemu_command) $(qemu_debug_interrupts) $(qemu_gdb_server) -kernel $(output_img)

gdb:
	@gdb-multiarch -quiet "$(output_elf)" -ex "set arch aarch64" -ex "target remote :1234"

clean:
	@cargo clean

copy-win:
	@PowerShell.exe -Command "copy $(output_img) D:/"
