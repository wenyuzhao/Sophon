include ../proton/common.mk

build:
	cargo build $(args) --target ../proton/aarch64-proton.json
	llvm-objdump --section-headers --source -d ../target/aarch64-proton/$(profile)/init > ../target/aarch64-proton/$(profile)/init.s 2> /dev/null