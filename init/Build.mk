include ../proton/common.mk

build:
	cargo build $(args) --target ../proton/aarch64-proton.json
	llvm-objdump --section-headers --source -d ../target/aarch64-proton/debug/init > ../target/aarch64-proton/debug/init.S 2> /dev/null