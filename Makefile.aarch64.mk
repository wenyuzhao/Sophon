# Run on QEMU
#     `make run`
# Build & deploy to a real raspi4 (Windows/WSL)
#     `raspi=4 make kernel objdump copy-win`

target = aarch64-proton
target_json = $(PWD)/src/arch/aarch64/aarch64-proton.json
output = kernel8.img
device ?= raspi3-qemu
profile = $(if $(release),release,debug)
debug_interrupts = $(if $(dint),-d int)



_output_elf = ./target/$(target)/$(profile)/proton_kernel
_output_init_elf = ./init/target/$(target)/$(profile)/init
_output_img = ./target/$(target)/$(profile)/kernel8.img
_qemu_command = qemu-system-aarch64 -display none -M raspi3 -serial stdio
_qemu_debug_interrupts = $(if $(dint),-d int)
_qemu_gdb_server = $(if $(gdb),-s -S)



kernel: init FORCE
	@cargo xbuild $(if $(release), --release) --target $(target_json) --features device-$(device)
	@cargo objcopy -- --strip-all $(_output_elf) -O binary $(_output_img)

init: FORCE
	@cd ./init && cargo xbuild $(if $(release), --release) --target aarch64-proton.json
	@mkdir -p ./target
	@cp ./init/target/$(target)/$(profile)/init ./target/init

run: device=raspi3-qemu
run: kernel objdump
	@$(_qemu_command) $(_qemu_debug_interrupts) $(_qemu_gdb_server) -kernel $(_output_img)

objdump:
	@llvm-objdump --source -d $(_output_elf) > kernel.S
	@llvm-objdump --source -d $(_output_init_elf) > init.S

gdb:
	@gdb-multiarch -quiet "$(_output_elf)" -ex "set arch aarch64" -ex "target remote :1234"

clean:
	@cargo clean
	@cd init && cargo clean
	@rm ./Cargo.lock ./kernel.S ./init.S ./kernel8.img

copy-win:
	@PowerShell.exe -Command "copy $(_output_img) D:/"

