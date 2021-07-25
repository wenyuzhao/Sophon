include ../sophon/common.mk

build:
	cargo build $(args) --target ../sophon/aarch64-sophon.json
	llvm-objdump --section-headers --source -d ../target/aarch64-sophon/$(profile)/init > ../target/aarch64-sophon/$(profile)/init.s 2> /dev/null